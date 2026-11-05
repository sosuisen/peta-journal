//! Holds the Crux core and runs its effects. What the user does comes in as [`Input`], what the
//! core asks for goes out through [`Host`], and each render tells Emacs what changed.
//!
//! Everything here runs on one thread: the requests the core waits on stay here, and the host's
//! threads only send back their results.

use crate::protocol::{
    AskKind, BacklinkEntry, Envelope, Inbound, Outbound, RowView, SearchEntry, SearchKind,
};
use crate::route::parse_url;
use crux_core::capability::Operation;
use crux_core::{Core, Request};
use peta_core::app::SelectionKey;
use peta_core::effects::{
    ApiOp, ApiResult, Effect, LocalDateTimeDto, SseEvent, SseOp, TimerOp, TimerOutput, UiOp,
    UiOutput,
};
use peta_core::util::filename::JournalType;
use peta_core::util::has_unclosed_code_fence;
use peta_core::view::{ScreenView, SearchView};
use peta_core::{Event, PetaApp, ViewModel};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};

/// The time as the core wants it.
pub struct Now {
    pub local: LocalDateTimeDto,
    pub now_ms: u64,
}

/// What the shell cannot do by itself. The results come back as [`Input`].
pub trait Host {
    fn emit(&mut self, message: Outbound);
    fn api(&mut self, id: u64, op: ApiOp);
    fn start_timer(&mut self, id: u64, ms: u32);
    fn connect_sse(&mut self, stream: u64);
    fn now(&self) -> Now;
}

pub enum Input {
    Emacs(Envelope),
    ApiDone {
        id: u64,
        result: Box<<ApiOp as Operation>::Output>,
    },
    TimerFired {
        id: u64,
    },
    Sse {
        stream: u64,
        event: SseEvent,
    },
}

/// What Emacs was told last, so it only hears about changes.
#[derive(Default)]
struct Sent {
    ack: u64,
    holding: bool,
    page_id: Option<String>,
    rows: Vec<RowView>,
    focus_seq: u64,
    connection: Option<String>,
    search: Option<(String, Vec<SearchEntry>)>,
    recent: Vec<String>,
    backlinks: Vec<BacklinkEntry>,
}

pub struct Shell<H: Host> {
    pub(crate) core: Core<PetaApp>,
    pub host: H,
    apis: HashMap<u64, Request<ApiOp>>,
    next_api: u64,
    timers: HashMap<u64, Request<TimerOp>>,
    sse: Option<(u64, Request<SseOp>)>,
    next_stream: u64,
    asks: HashMap<u64, (AskKind, Request<UiOp>)>,
    next_ask: u64,
    /// The line of Emacs being handled.
    ack: u64,
    sent: Sent,
    /// The edit Emacs itself just started. It already has the point there, and answering would
    /// pull back a point that has moved on since.
    own_edit: Option<(String, usize)>,
    /// What waits for the next render to reach Emacs first.
    after_render: Vec<Outbound>,
    /// The URLs the core asked for, opened after its effects are done.
    navigations: VecDeque<(String, bool)>,
    current_url: Option<String>,
    history: Vec<String>,
}

impl<H: Host> Shell<H> {
    pub fn new(host: H) -> Shell<H> {
        Shell {
            core: Core::new(),
            host,
            apis: HashMap::new(),
            next_api: 0,
            timers: HashMap::new(),
            sse: None,
            next_stream: 0,
            asks: HashMap::new(),
            next_ask: 0,
            ack: 0,
            sent: Sent::default(),
            own_edit: None,
            after_render: Vec::new(),
            navigations: VecDeque::new(),
            current_url: None,
            history: Vec::new(),
        }
    }

    pub fn handle(&mut self, input: Input) {
        match input {
            Input::Emacs(Envelope { seq, message }) => {
                self.ack = seq;
                self.on_emacs(message);
            }
            Input::ApiDone { id, result } => self.on_api_done(id, *result),
            Input::TimerFired { id } => {
                // A timer cancelled in the meantime is gone from the map
                if let Some(mut request) = self.timers.remove(&id) {
                    self.resolve(&mut request, TimerOutput::Fired);
                }
            }
            Input::Sse { stream, event } => self.on_sse(stream, event),
        }
        self.tell_state();
    }

    /// Ends the edit, so typed text is saved. The caller waits for [`Shell::is_idle`].
    pub fn shutdown(&mut self) {
        let focused = self.core.view().outline.focused_item_id;
        if let Some(item_id) = focused {
            self.send(Event::EditFinished { item_id });
        }
    }

    /// True when nothing that could still save something is under way.
    pub fn is_idle(&self) -> bool {
        self.apis.is_empty() && self.timers.is_empty()
    }

    fn on_api_done(&mut self, id: u64, result: ApiResult<peta_core::effects::ApiOutput>) {
        if let Some(mut request) = self.apis.remove(&id) {
            self.resolve(&mut request, result);
        }
    }

    fn on_sse(&mut self, stream: u64, event: SseEvent) {
        let Some((current, mut request)) = self.sse.take() else {
            return;
        };
        if current != stream {
            self.sse = Some((current, request));
            return;
        }
        self.resolve(&mut request, event);
        // The answer may have made the core reconnect, which replaces the stream
        if self.sse.is_none() {
            self.sse = Some((current, request));
        }
    }

    fn on_emacs(&mut self, message: Inbound) {
        match message {
            Inbound::Start { url } => {
                self.send(Event::Start);
                self.navigate(&url, false);
            }
            Inbound::Go { url } => self.navigate(&url, false),
            Inbound::Back => self.back(),
            Inbound::GoToPage { path } => self.send(Event::GoToPage {
                path,
                new_tab: false,
                new_window: false,
            }),
            Inbound::OpenWikiLink { text } => self.send(Event::OpenWikiLink { text }),
            Inbound::OpenTag { tag } => {
                let tags_dir = self.core.view().config.tags_dir;
                self.navigate(&format!("/pages/{tags_dir}{tag}"), false);
            }
            Inbound::JournalNav { forward } => self.journal_nav(forward),
            Inbound::EditRequested { id, cursor } => {
                self.own_edit = Some((id.clone(), cursor));
                self.send(Event::EditRequested {
                    item_id: id,
                    cursor,
                });
                self.own_edit = None;
            }
            Inbound::EditFinished { id } => self.send(Event::EditFinished { item_id: id }),
            Inbound::BodyChanged { id, body } => self.body_changed(id, body),
            Inbound::Enter { id, cursor } => self.enter(id, cursor),
            Inbound::BackspaceAtStart { id } => {
                self.send(Event::BackspaceAtStart { item_id: id });
            }
            Inbound::DeleteAtEnd { id } => self.send(Event::DeleteAtEnd { item_id: id }),
            Inbound::Indent { id } => self.send(Event::Indent { item_id: id }),
            Inbound::Outdent { id } => self.send(Event::Outdent { item_id: id }),
            Inbound::MoveUp { id } => self.send(Event::MoveUp { item_id: id }),
            Inbound::MoveDown { id } => self.send(Event::MoveDown { item_id: id }),
            Inbound::TaskCycle { id } => self.send(Event::TaskCycle { item_id: id }),
            Inbound::CollapseToggle { id } => self.send(Event::CollapseToggle { item_id: id }),
            Inbound::Paste { id, text, cursor } => self.send(Event::PasteText {
                item_id: id,
                text,
                html: String::new(),
                cursor,
                raw: false,
            }),
            Inbound::DeleteItem { id } => {
                self.send(Event::ShiftSelect { item_id: id });
                self.send(Event::SelectionKey(SelectionKey::Delete));
            }
            Inbound::Undo => self.send(Event::Undo),
            Inbound::Redo => self.send(Event::Redo),
            Inbound::Scrolled { top, bottom } => self.send(Event::OutlineScrolled {
                scrolled_from_top: top > 0,
                top_visible_index: top,
                rendered_start: top,
                rendered_end: bottom,
            }),
            Inbound::Search { query } => {
                self.send(Event::SearchDropdown(true));
                self.send(Event::SearchQueryChanged(query));
            }
            Inbound::SearchResult { index } => self.send(Event::SearchResultClicked { index }),
            Inbound::Answer { req, value } => self.answer(req, value),
        }
    }

    fn body_changed(&mut self, id: String, body: String) {
        // Emacs has this text already: the render below must not send it back
        if let Some(row) = self.sent.rows.iter_mut().find(|r| r.id == id) {
            row.body = body.clone();
        }
        let now_ms = self.host.now().now_ms;
        self.send(Event::BodyChanged {
            item_id: id,
            body,
            now_ms,
        });
    }

    fn enter(&mut self, id: String, cursor: usize) {
        let view = self.core.view();
        let body = view
            .outline
            .rows
            .iter()
            .find(|r| r.item.id == id)
            .map(|r| r.item.body.clone())
            .unwrap_or_default();
        let before: String = body.chars().take(cursor).collect();
        if has_unclosed_code_fence(&before) {
            let after: String = body.chars().skip(cursor).collect();
            let now_ms = self.host.now().now_ms;
            self.after_render.push(Outbound::Focus {
                ack: self.ack,
                id: id.clone(),
                cursor: Some(cursor + 1),
            });
            // Not through `body_changed`: Emacs does not have this newline yet
            self.send(Event::BodyChanged {
                item_id: id,
                body: format!("{before}\n{after}"),
                now_ms,
            });
            return;
        }
        let now_ms = self.host.now().now_ms;
        self.send(Event::EnterPressed {
            item_id: id,
            cursor,
            now_ms,
        });
    }

    fn journal_nav(&mut self, forward: bool) {
        use peta_core::app::JournalNav;
        let weekly = matches!(
            self.core.view().screen,
            ScreenView::Journal(j) if j.journal_type == Some(JournalType::Weekly)
        );
        self.send(Event::JournalNav(match (weekly, forward) {
            (false, false) => JournalNav::PrevDay,
            (false, true) => JournalNav::NextDay,
            (true, false) => JournalNav::PrevWeek,
            (true, true) => JournalNav::NextWeek,
        }));
    }

    fn answer(&mut self, req: u64, value: Value) {
        let Some((kind, mut request)) = self.asks.remove(&req) else {
            return;
        };
        match kind {
            AskKind::Confirm | AskKind::ConfirmDestructive => {
                let ok = value.as_bool().unwrap_or(false);
                self.resolve(&mut request, UiOutput::Bool(ok));
                // The deletion that asked has selected its items. It ends here, so they must
                // not stay selected: an editor does not open while they are
                if !ok && self.core.view().outline.selection_mode {
                    self.send(Event::SelectionKey(SelectionKey::Escape));
                }
            }
            AskKind::Prompt | AskKind::Clipboard => {
                let text = value.as_str().map(str::to_string);
                self.resolve(&mut request, UiOutput::Text(text));
            }
        }
    }

    fn navigate(&mut self, url: &str, replace: bool) {
        let Some(route) = parse_url(url) else {
            return;
        };
        if !replace {
            if let Some(current) = self.current_url.take() {
                self.history.push(current);
            }
        }
        self.current_url = Some(url.to_string());
        self.open_route(route);
    }

    fn back(&mut self) {
        let Some(url) = self.history.pop() else {
            return;
        };
        if let Some(route) = parse_url(&url) {
            self.current_url = Some(url);
            self.open_route(route);
        }
    }

    fn open_route(&mut self, route: peta_core::Route) {
        let local = self.host.now().local;
        let today = format!("{:04}-{:02}-{:02}", local.year, local.month, local.day);
        self.send(Event::Route { route, today });
    }

    fn send(&mut self, event: Event) {
        let effects = self.core.process_event(event);
        self.process(effects);
    }

    fn resolve<Op: Operation>(&mut self, request: &mut Request<Op>, output: Op::Output) {
        match self.core.resolve(request, output) {
            Ok(effects) => self.process(effects),
            Err(e) => eprintln!("[shell-emacs] resolve failed: {e}"),
        }
    }

    fn process(&mut self, effects: Vec<Effect>) {
        let mut render = false;
        let mut focused = Vec::new();
        for effect in effects {
            match effect {
                Effect::Render(_) => render = true,
                Effect::Api(request) => {
                    self.next_api += 1;
                    self.host.api(self.next_api, request.operation.clone());
                    self.apis.insert(self.next_api, request);
                }
                Effect::Timer(request) => self.on_timer_op(request),
                Effect::Sse(request) => {
                    self.next_stream += 1;
                    self.sse = Some((self.next_stream, request));
                    self.host.connect_sse(self.next_stream);
                }
                Effect::Ui(request) => {
                    if let Some(request) = self.on_ui_op(request) {
                        focused.push(request);
                    }
                }
                // A notification: the core does not wait for it
                Effect::Navigate(request) => {
                    let op = &request.operation;
                    self.navigations.push_back((op.url.clone(), op.replace));
                }
            }
        }
        if render {
            self.tell_view();
        }
        for message in std::mem::take(&mut self.after_render) {
            self.host.emit(message);
        }
        // After the render: the item to focus may have been added in this very update
        for mut request in focused {
            self.resolve(&mut request, UiOutput::Done);
        }
        while let Some((url, replace)) = self.navigations.pop_front() {
            self.navigate(&url, replace);
        }
    }

    fn on_timer_op(&mut self, request: Request<TimerOp>) {
        match request.operation {
            TimerOp::Start { id, ms } => {
                self.timers.insert(id, request);
                self.host.start_timer(id, ms);
            }
            TimerOp::Cancel { id } => {
                if let Some(mut running) = self.timers.remove(&id) {
                    self.resolve(&mut running, TimerOutput::Cancelled);
                }
            }
        }
    }

    /// Performs a screen operation. A focus request comes back, to be resolved after the render.
    fn on_ui_op(&mut self, mut request: Request<UiOp>) -> Option<Request<UiOp>> {
        match request.operation.clone() {
            // The keys are never held in the core: Emacs keeps them in its own input queue while
            // it waits for this answer, so `replay_keys` stays empty
            UiOp::FocusEditor { item_id, .. } => {
                if let Some(id) = item_id {
                    self.after_render.push(Outbound::Focus {
                        ack: self.ack,
                        id,
                        cursor: None,
                    });
                }
                return Some(request);
            }
            UiOp::ScrollToItem { item_id, .. } => {
                self.after_render.push(Outbound::Scroll { id: item_id });
            }
            UiOp::SetTitle(title) => self.host.emit(Outbound::Title { title }),
            UiOp::CopyText(text) | UiOp::CopyMarkdown { markdown: text } => {
                self.host.emit(Outbound::Copy { text });
            }
            UiOp::Alert(text) => self.after_render.push(Outbound::Message { text }),
            UiOp::Confirm(message) => self.ask(AskKind::Confirm, message, request),
            UiOp::ConfirmDestructive(message) => {
                self.ask(AskKind::ConfirmDestructive, message, request);
            }
            UiOp::Prompt(message) => self.ask(AskKind::Prompt, message, request),
            UiOp::ReadClipboardText => self.ask(AskKind::Clipboard, String::new(), request),
            UiOp::LocalNow => {
                let now = self.host.now();
                self.resolve(
                    &mut request,
                    UiOutput::Now {
                        local: now.local,
                        now_ms: now.now_ms,
                    },
                );
            }
            // Emacs has one window on one page, so another window is this one
            UiOp::OpenWindow { url, .. } => self.navigations.push_back((url, false)),
            // There is nothing to do these with here: the atlas map and the extension iframes
            // are parts of the web shell, and a clipboard write is never begun ahead of its text
            UiOp::SaveAtlasDirs(_)
            | UiOp::CopyFailed
            | UiOp::LoadExtensions { .. }
            | UiOp::ExtensionPageCreate { .. }
            | UiOp::ExtensionPageTagAdd { .. }
            | UiOp::ExtensionReply { .. } => {}
        }
        None
    }

    fn ask(&mut self, kind: AskKind, message: String, request: Request<UiOp>) {
        self.next_ask += 1;
        self.asks.insert(self.next_ask, (kind, request));
        self.after_render.push(Outbound::Ask {
            req: self.next_ask,
            kind,
            message,
        });
    }

    /// Tells Emacs what changed in the view.
    fn tell_view(&mut self) {
        let view = self.core.view();
        self.tell_rows(&view);
        self.tell_focus(&view);
        self.tell_connection(&view);
        self.tell_search(&view.search);
        self.tell_recent(&view);
        self.tell_backlinks(&view);
        self.sent.holding = view.outline.key_relay_holding;
    }

    fn tell_rows(&mut self, view: &ViewModel) {
        let rows: Vec<RowView> = view.outline.rows.iter().map(RowView::of).collect();
        let page_id = view.sidebar.current_page_id.clone();
        if rows == self.sent.rows && page_id == self.sent.page_id {
            return;
        }
        self.sent.rows.clone_from(&rows);
        self.sent.page_id.clone_from(&page_id);
        self.host.emit(Outbound::Rows {
            ack: self.ack,
            page_id,
            rows,
        });
    }

    fn tell_focus(&mut self, view: &ViewModel) {
        let Some(request) = &view.outline.focus_request else {
            return;
        };
        if request.seq == self.sent.focus_seq {
            return;
        }
        self.sent.focus_seq = request.seq;
        let own = self
            .own_edit
            .as_ref()
            .is_some_and(|(id, cursor)| *id == request.item_id && Some(*cursor) == request.cursor);
        if !own {
            self.host.emit(Outbound::Focus {
                ack: self.ack,
                id: request.item_id.clone(),
                cursor: request.cursor,
            });
        }
    }

    fn tell_connection(&mut self, view: &ViewModel) {
        let connection = view.connection.as_str().to_string();
        if self.sent.connection.as_ref() != Some(&connection) {
            self.sent.connection = Some(connection.clone());
            self.host.emit(Outbound::Status { connection });
        }
    }

    fn tell_search(&mut self, search: &SearchView) {
        let entries = search_entries(search);
        let now = Some((search.query.clone(), entries));
        if self.sent.search != now {
            self.sent.search.clone_from(&now);
            if let Some((query, results)) = now {
                self.host.emit(Outbound::Search { query, results });
            }
        }
    }

    fn tell_recent(&mut self, view: &ViewModel) {
        let paths: Vec<String> = view
            .sidebar
            .recent_pages
            .iter()
            .map(|p| p.path.clone())
            .collect();
        if paths != self.sent.recent {
            self.sent.recent.clone_from(&paths);
            self.host.emit(Outbound::Recent { paths });
        }
    }

    fn tell_backlinks(&mut self, view: &ViewModel) {
        let groups: Vec<BacklinkEntry> = match &view.screen {
            ScreenView::Page(Some(page)) => {
                page.backlinks.iter().map(BacklinkEntry::from).collect()
            }
            _ => Vec::new(),
        };
        if groups != self.sent.backlinks {
            self.sent.backlinks.clone_from(&groups);
            self.host.emit(Outbound::Backlinks { groups });
        }
    }

    /// Tells Emacs which line was handled, and whether the core still holds keys.
    fn tell_state(&mut self) {
        let holding = self.core.view().outline.key_relay_holding;
        if self.ack != self.sent.ack || holding != self.sent.holding {
            self.sent.ack = self.ack;
            self.sent.holding = holding;
            self.host.emit(Outbound::State {
                ack: self.ack,
                holding,
            });
        }
    }
}

/// The results as the core numbers them: the pages, what can be created, the items.
fn search_entries(search: &SearchView) -> Vec<SearchEntry> {
    let pages = search
        .page_results
        .iter()
        .map(|r| (SearchKind::Page, r.page_title.clone(), r.snippet.clone()));
    let creates = search
        .create_titles
        .iter()
        .map(|t| (SearchKind::Create, t.clone(), String::new()));
    let items = search
        .item_results
        .iter()
        .map(|r| (SearchKind::Item, r.page_title.clone(), r.snippet.clone()));
    pages
        .chain(creates)
        .chain(items)
        .enumerate()
        .map(|(index, (kind, title, snippet))| SearchEntry {
            index,
            kind,
            title,
            snippet,
        })
        .collect()
}
