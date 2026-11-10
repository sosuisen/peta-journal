//! Holds the Crux core and runs its effects. What the user does comes in as [`Input`], what the
//! core asks for goes out through [`Host`], and each render tells Emacs what changed.
//!
//! Everything here runs on one thread: the requests the core waits on stay here, and the host's
//! threads only send back their results.

use crate::protocol::{BacklinkEntry, Envelope, Inbound, Outbound, PropertyRow, RowView};
use crate::route::parse_url;
use crux_core::capability::Operation;
use crux_core::{Core, Request};
use peta_core::app::JournalNav;
use peta_core::effects::{
    ApiOp, ApiResult, Effect, LocalDateTimeDto, SseEvent, SseOp, TimerOp, TimerOutput, UiOp,
    UiOutput,
};
use peta_core::rest::ClientId;
use peta_core::util::{auto_close, link_at};
use peta_core::view::{ScreenView, SearchView};
use peta_core::{Event, PetaApp, ViewModel};
use std::collections::{HashMap, HashSet, VecDeque};

/// The time as the core wants it.
pub struct Now {
    pub local: LocalDateTimeDto,
    pub now_ms: u64,
}

/// What the shell cannot do by itself. The results come back as [`Input`].
pub trait Host {
    fn emit(&mut self, message: Outbound);
    fn api(&mut self, id: u64, op: ApiOp);
    /// Sends the file at `path` to the server. The result comes back like that of [`Host::api`].
    fn upload(&mut self, id: u64, path: String);
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
    search: Option<(Vec<peta_core::view::SearchEntry>, bool)>,
    page_path: Option<String>,
    properties: (Option<Vec<PropertyRow>>, Vec<String>, Vec<String>),
    recent: Vec<String>,
    backlinks: Vec<BacklinkEntry>,
}

pub struct Shell<H: Host> {
    pub(crate) core: Core<PetaApp>,
    pub host: H,
    client_id: ClientId,
    apis: HashMap<u64, Request<ApiOp>>,
    next_api: u64,
    /// The paths of the images Emacs chose. The core knows each by its index here.
    image_files: Vec<String>,
    /// The calls under way for an image: its upload, then the save of its link.
    image_calls: HashSet<u64>,
    timers: HashMap<u64, Request<TimerOp>>,
    sse: Option<(u64, Request<SseOp>)>,
    next_stream: u64,
    asks: HashMap<u64, Request<UiOp>>,
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
    /// `client_id` is the ID that the host sends with every request (ADR-015).
    pub fn new(host: H, client_id: ClientId) -> Shell<H> {
        Shell {
            core: Core::new(),
            host,
            client_id,
            apis: HashMap::new(),
            next_api: 0,
            image_files: Vec::new(),
            image_calls: HashSet::new(),
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

    /// The ID this shell names itself with, to the server and to the core.
    pub fn client_id(&self) -> &ClientId {
        &self.client_id
    }

    /// True when nothing that could still save something is under way.
    pub fn is_idle(&self) -> bool {
        self.apis.is_empty() && self.timers.is_empty()
    }

    fn on_api_done(&mut self, id: u64, result: ApiResult<peta_core::effects::ApiOutput>) {
        let Some(mut request) = self.apis.remove(&id) else {
            return;
        };
        if !self.image_calls.remove(&id) {
            self.resolve(&mut request, result);
            return;
        }
        // The core only writes this to its log, and the image would just not appear
        if let Err(e) = &result {
            self.host.emit(Outbound::Message {
                text: format!("Could not insert the image: {e}"),
            });
        }
        self.keeping_image_calls(|shell| shell.resolve(&mut request, result));
    }

    /// Counts the calls that `step` starts as calls for an image.
    fn keeping_image_calls(&mut self, step: impl FnOnce(&mut Self)) {
        let before = self.next_api;
        step(self);
        self.image_calls.extend(before + 1..=self.next_api);
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
                self.send(Event::Start {
                    client_id: self.client_id.clone(),
                });
                self.navigate(&url, false);
            }
            Inbound::Go { url } => self.navigate(&url, false),
            Inbound::Back => self.back(),
            Inbound::GoToPage { path } => self.send(Event::GoToPage {
                path,
                new_tab: false,
                new_window: false,
            }),
            // The whole path goes in the name: the core takes the folders out of it at the `/`
            Inbound::RenamePage { path } => self.send(Event::PageTitleSave {
                dir_path: String::new(),
                file_name: path,
            }),
            Inbound::SetProperty { key, value } => self.set_property(key, value),
            Inbound::DeleteProperty { key } => self.send(Event::DeletePageProperty { key }),
            Inbound::OpenLinkAt { id, cursor } => self.open_link_at(id, cursor),
            Inbound::JournalNav { forward } => self.send(Event::JournalNav(if forward {
                JournalNav::Next
            } else {
                JournalNav::Prev
            })),
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
            Inbound::Typed { id, cursor } => self.typed(id, cursor),
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
            Inbound::InsertImage { id, path, cursor } => self.insert_image(id, path, cursor),
            Inbound::DeleteItem { id } => self.send(Event::DeleteItem { item_id: id }),
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
            Inbound::Answer { req, yes } => self.answer(req, yes),
        }
    }

    fn insert_image(&mut self, id: String, path: String, cursor: usize) {
        let file = self.image_files.len() as u64;
        self.image_files.push(path);
        self.keeping_image_calls(|shell| {
            shell.send(Event::PasteImage {
                item_id: id,
                file,
                cursor,
            });
        });
    }

    fn set_property(&mut self, key: String, value: String) {
        let value_type = property_type(&self.core.view(), &key);
        self.send(Event::SetPageProperty {
            key,
            value_type,
            value,
        });
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

    fn typed(&mut self, id: String, cursor: usize) {
        let closed = self
            .core
            .view()
            .outline
            .rows
            .iter()
            .find(|r| r.item.id == id)
            .and_then(|r| auto_close(&r.item.body, cursor));
        let Some((body, cursor)) = closed else {
            return;
        };
        let now_ms = self.host.now().now_ms;
        self.after_render.push(Outbound::Focus {
            ack: self.ack,
            id: id.clone(),
            cursor: Some(cursor),
        });
        // Not through `body_changed`: Emacs does not have this text yet
        self.send(Event::BodyChanged {
            item_id: id,
            body,
            now_ms,
        });
    }

    fn enter(&mut self, id: String, cursor: usize) {
        let now_ms = self.host.now().now_ms;
        self.send(Event::EnterPressed {
            item_id: id,
            cursor,
            now_ms,
        });
    }

    fn open_link_at(&mut self, id: String, cursor: usize) {
        // Only to say so: a key that seems to do nothing would look broken in Emacs
        let on_link = self
            .core
            .view()
            .outline
            .rows
            .iter()
            .find(|r| r.item.id == id)
            .is_some_and(|r| link_at(&r.item.body, cursor).is_some());
        if !on_link {
            self.host.emit(Outbound::Message {
                text: "No link here".into(),
            });
        }
        self.send(Event::OpenLinkAt {
            item_id: id,
            cursor,
        });
    }

    fn answer(&mut self, req: u64, yes: bool) {
        let Some(mut request) = self.asks.remove(&req) else {
            return;
        };
        self.resolve(&mut request, UiOutput::Bool(yes));
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
                    match &request.operation {
                        ApiOp::UploadAsset { file } => {
                            let path = self.image_files[*file as usize].clone();
                            self.host.upload(self.next_api, path);
                        }
                        op => self.host.api(self.next_api, op.clone()),
                    }
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
            UiOp::Alert(text) => self.after_render.push(Outbound::Message { text }),
            UiOp::Confirm(message) | UiOp::ConfirmDestructive(message) => {
                self.ask(message, request);
            }
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
            // There is nothing to do these with here: the atlas map is a part of the web shell.
            // Copying, a new page from the header and pasting a transclusion start from events
            // that Emacs does not send
            UiOp::SaveAtlasDirs(_)
            | UiOp::CopyText(_)
            | UiOp::CopyMarkdown { .. }
            | UiOp::CopyFailed
            | UiOp::Prompt(_)
            | UiOp::ReadClipboardText => {}
        }
        None
    }

    fn ask(&mut self, message: String, request: Request<UiOp>) {
        self.next_ask += 1;
        self.asks.insert(self.next_ask, request);
        self.after_render.push(Outbound::Ask {
            req: self.next_ask,
            message,
        });
    }

    /// Tells Emacs what changed in the view.
    fn tell_view(&mut self) {
        let view = self.core.view();
        self.tell_rows(&view);
        self.tell_focus(&view);
        self.tell_page_path(&view);
        self.tell_properties(&view);
        self.tell_connection(&view);
        self.tell_search(&view.search);
        self.tell_recent(&view);
        self.tell_backlinks(&view);
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

    fn tell_page_path(&mut self, view: &ViewModel) {
        let path = match &view.screen {
            ScreenView::Page(Some(page)) if !page.is_index && !page.is_atlas => {
                Some(page.page.canonical_path())
            }
            _ => None,
        };
        if path != self.sent.page_path {
            self.sent.page_path.clone_from(&path);
            self.host.emit(Outbound::PagePath { path });
        }
    }

    fn tell_properties(&mut self, view: &ViewModel) {
        let rows: Option<Vec<PropertyRow>> = match &view.screen {
            ScreenView::Page(Some(page)) => {
                Some(page.properties.iter().map(PropertyRow::from).collect())
            }
            _ => None,
        };
        let is_used = |key: &str| rows.iter().flatten().any(|r| r.key == key);
        let free_keys: Vec<String> = view
            .config
            .property_definitions
            .iter()
            .filter(|d| d.page && !is_used(&d.key))
            .map(|d| d.key.clone())
            .collect();
        let datetime_keys = datetime_keys(view, rows.iter().flatten(), &free_keys);
        let now = (rows, free_keys, datetime_keys);
        if now != self.sent.properties {
            self.sent.properties.clone_from(&now);
            let (rows, free_keys, datetime_keys) = now;
            self.host.emit(Outbound::Properties {
                rows,
                free_keys,
                datetime_keys,
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
        let now = Some((search.results.clone(), search.searching));
        if self.sent.search != now {
            self.sent.search.clone_from(&now);
            if let Some((results, searching)) = now {
                self.host.emit(Outbound::Search { results, searching });
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
        // The core writes the link into the body it had at the start of the insertion: a key typed
        // in the meantime would be overwritten
        let holding = self.core.view().outline.key_relay_holding || !self.image_calls.is_empty();
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

/// The keys whose value is a date and a time. Only the page's own dates cannot be removed, and
/// they are dates without a definition.
fn datetime_keys<'a>(
    view: &ViewModel,
    rows: impl Iterator<Item = &'a PropertyRow>,
    free_keys: &[String],
) -> Vec<String> {
    let is_datetime = |key: &str| property_type(view, key) == "datetime";
    rows.filter(|row| !row.removable || is_datetime(&row.key))
        .map(|row| row.key.clone())
        .chain(free_keys.iter().filter(|key| is_datetime(key)).cloned())
        .collect()
}

/// The type to send a value of the key with. A list stays a list, even when it has no definition.
fn property_type(view: &ViewModel, key: &str) -> String {
    let is_list = match &view.screen {
        ScreenView::Page(Some(page)) => page.properties.iter().any(|p| p.key == key && p.is_list),
        _ => false,
    };
    if is_list {
        return "tags".to_string();
    }
    view.config
        .property_definitions
        .iter()
        .find(|d| d.key == key)
        .map_or_else(|| "text".to_string(), |d| d.value_type.clone())
}
