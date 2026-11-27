//! Holds the Crux core and runs its effects. What the user does comes in as [`Input`], what the
//! core asks for goes out through [`Host`], and each render tells Emacs what changed.
//!
//! Everything here runs on one thread: the requests the core waits on stay here, and the host's
//! threads only send back their results.

use crate::emacs_text::{EmacsText, SharedText};
use crate::protocol::{
    BacklinkEntry, DateUnit, Envelope, FreeKey, Inbound, Outbound, PropertyRow, RowChange, RowView,
    SearchEntry, SidebarView,
};
use client_core::app::{JournalNav, WikiLinkContext};
use client_core::changes::{list_changes, Panel, ShownPanels};
use client_core::effects::{
    ApiOp, ApiOutput, ApiResult, Effect, LocalDateTimeDto, SseEvent, SseOp, TimerOp, TimerOutput,
    UiOp, UiOutput,
};
use client_core::keys::Held;
use client_core::model::{bare_name, value_type_of, PageExt};
use client_core::policy::{InOrder, Timers};
use client_core::rest::ClientId;
use client_core::screens::calendar::{month_page_name, year_page_name};
use client_core::screens::day::CHECK_MS;
use client_core::util::date::Date;
use client_core::util::link_at;
use client_core::util::week::week_page_name;
use client_core::view::{Row, ScreenView, SearchView};
use client_core::{EditPlace, Event, PetaApp, ViewModel};
use crux_core::capability::Operation;
use crux_core::{Core, Request};
use std::collections::{HashMap, HashSet, VecDeque};

/// The time as the core wants it.
pub struct Now {
    pub local: LocalDateTimeDto,
    pub now_ms: u64,
}

/// What the shell cannot do by itself. The results come back as [`Input`].
pub trait Host {
    fn emit(&mut self, message: Outbound);
    /// Asks the server. The host sends its calls and uploads one at a time, in the order the
    /// shell gave them (`client_core::policy`).
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
    rows: Vec<SentRow>,
    focus_seq: u64,
    connection: Option<String>,
    search: Option<(Vec<client_core::view::SearchEntry>, bool)>,
    page_path: Option<String>,
    index: (Option<String>, Option<String>),
    properties: (Option<Vec<PropertyRow>>, Vec<FreeKey>, Vec<String>),
    recent: Vec<String>,
    sidebar: SidebarView,
    backlinks: Vec<BacklinkEntry>,
}

/// A row as Emacs was told of it. Its text is shared with the view, and is copied into the
/// row only when the row goes to Emacs: a row that did not change is compared, not copied
/// (#960).
#[derive(Clone, PartialEq)]
struct SentRow {
    /// The row with an empty text.
    row: RowView,
    text: SharedText,
}

impl SentRow {
    fn of(row: &Row<SharedText>) -> SentRow {
        SentRow {
            row: RowView::of(row, EmacsText::default()),
            text: row.body_model.clone(),
        }
    }

    fn into_view(self) -> RowView {
        RowView {
            text: EmacsText::clone(&self.text.0),
            ..self.row
        }
    }
}

pub struct Shell<H: Host> {
    pub(crate) core: Core<PetaApp<SharedText>>,
    pub host: H,
    client_id: ClientId,
    apis: HashMap<u64, Request<ApiOp>>,
    next_api: u64,
    /// The text Emacs took from its clipboard, while the core may ask for it.
    clipboard: Option<String>,
    /// The paths of the images Emacs chose. The core knows each by its index here.
    image_files: Vec<String>,
    /// The calls under way for an image: its upload, then the paste of its link.
    image_calls: HashSet<u64>,
    timers: Timers<Request<TimerOp>>,
    /// The timer that waits to look at the clock again. It runs all the time and saves
    /// nothing, so the shell is idle while only it runs.
    clock_watch: Option<u64>,
    sse: Option<(u64, Request<SseOp>)>,
    next_stream: u64,
    /// The questions to the user with their messages, each waiting for the answer to the one
    /// before (`client_core::policy`): a second question in the minibuffer takes the place of
    /// the first.
    asks: InOrder<(String, Request<UiOp>)>,
    /// The question Emacs has now, by the number it answers with.
    asked: Option<(u64, Request<UiOp>)>,
    next_ask: u64,
    /// The line of Emacs being handled.
    ack: u64,
    sent: Sent,
    /// The panels of the view when Emacs was last told.
    panels: ShownPanels,
    /// Whether the core held keys at the last render. It is read from there, and not from a
    /// view made again: the core changes it only in an update, and every update renders.
    relay_holding: bool,
    /// The edit Emacs itself just started. It already has the point there, and answering would
    /// pull back a point that has moved on since.
    own_edit: Option<(String, usize)>,
    /// What waits for the next render to reach Emacs first.
    after_render: Vec<Outbound>,
    /// The URLs the core asked for, opened after its effects are done.
    navigations: VecDeque<String>,
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
            clipboard: None,
            image_files: Vec::new(),
            image_calls: HashSet::new(),
            timers: Timers::default(),
            clock_watch: None,
            sse: None,
            next_stream: 0,
            asks: InOrder::new(),
            asked: None,
            next_ask: 0,
            ack: 0,
            sent: Sent::default(),
            panels: ShownPanels::default(),
            relay_holding: false,
            own_edit: None,
            after_render: Vec::new(),
            navigations: VecDeque::new(),
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
                if let Some(mut request) = self.timers.end(id) {
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
        self.apis.is_empty() && self.timers.ids().all(|id| Some(id) == self.clock_watch)
    }

    fn on_api_done(&mut self, id: u64, result: ApiResult<ApiOutput>) {
        // The host answers each call once
        let mut request = self.apis.remove(&id).expect("the call waits");
        if let ApiOp::SearchPageNames { query, .. } = &request.operation {
            self.host.emit(Outbound::PageNames {
                query: query.clone(),
                titles: page_titles(&result),
            });
        }
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
        // The host only has a stream to tell about after the core asked for one
        let (current, mut request) = self.sse.take().expect("a stream is connected");
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
                self.open(url);
            }
            // The name is made a URL here, so that Emacs does not need the rule of encoding
            Inbound::StartAtPage { path } => self.on_emacs(Inbound::Start {
                url: page_urls::page_url(&path),
            }),
            Inbound::Go { url } => self.open(url),
            Inbound::Back => {
                let today = self.today();
                self.send(Event::Back { today });
            }
            Inbound::Forward => {
                let today = self.today();
                self.send(Event::Forward { today });
            }
            Inbound::GoToPage { path } => self.send(Event::GoToPage {
                path,
                open_in: client_core::keys::OpenIn::Here,
            }),
            Inbound::GoToDate { date, unit } => self.go_to_date(&date, unit),
            // The whole path goes in the name: the core takes the folders out of it at the `/`
            Inbound::RenamePage { path } => self.send(Event::PageTitleSave {
                dir_path: String::new(),
                file_name: path,
            }),
            Inbound::RenameFolder { path } => self.send(Event::PageTitleSave {
                dir_path: path,
                file_name: "index".into(),
            }),
            Inbound::DemoteIndex => self.send(Event::DemoteIndex),
            Inbound::DeletePage => self.send(Event::DeleteCurrentPage),
            Inbound::DayMayHaveChanged => self.send(Event::DateMayHaveChanged),
            Inbound::SetProperty { key, value } => self.set_property(key, value),
            Inbound::DeleteProperty { key } => self.send(Event::DeletePageProperty { key }),
            Inbound::OpenLinkAt { id, cursor } => self.open_link_at(id, cursor),
            // Emacs has no new view, so a click holds no keys (ADR 083)
            Inbound::LinkClicked { link } => self.send(Event::LinkClicked {
                link,
                held: Held::default(),
            }),
            Inbound::JournalNav { forward } => self.send(Event::JournalNav(if forward {
                JournalNav::Next
            } else {
                JournalNav::Prev
            })),
            Inbound::EditRequested { id, cursor } => {
                self.own_edit = Some((id.clone(), cursor));
                self.send(Event::EditRequested {
                    item_id: id,
                    place: EditPlace::Markdown(cursor),
                });
                self.own_edit = None;
            }
            Inbound::EditFinished { id } => self.send(Event::EditFinished { item_id: id }),
            Inbound::EditAtLetter { id, letter } => self.send(Event::EditRequested {
                item_id: id,
                place: EditPlace::ShownLetter(letter),
            }),
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
            Inbound::Select { id } => self.send(Event::ShiftSelect { item_id: id }),
            Inbound::SelectionKey { key } => self.send(Event::SelectionKey(key.into())),
            Inbound::PasteAsTransclusion { id, text } => {
                // The core asks for the clipboard text, and gets this one
                self.clipboard = Some(text);
                self.send(Event::PasteAsTransclusion { item_id: id });
                self.clipboard = None;
            }
            Inbound::RemoveTransclusion { id } => {
                self.send(Event::RemoveTransclusion { item_id: id })
            }
            Inbound::SendToStash { id } => self.send(Event::SendToStash { item_id: id }),
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
            Inbound::SearchResult { position } => {
                self.send(Event::SearchResultClicked { position })
            }
            Inbound::Answer { req, yes } => self.answer(req, yes),
            // The popup of the web frontend has a place on the screen. Emacs shows the names
            // with its own completion, so the place is not used
            Inbound::PageNames { id, query } => self.send(Event::WikiQuery {
                item_id: id,
                context: Some(WikiLinkContext {
                    query,
                    x: 0.0,
                    y: 0.0,
                }),
            }),
        }
    }

    fn go_to_date(&mut self, date: &str, unit: DateUnit) {
        let date = Date::parse(date).expect("Emacs sends a date of its calendar");
        let path = match unit {
            DateUnit::Day => date.iso(),
            DateUnit::Week => week_page_name(date, self.core.view().config.week_numbering),
            DateUnit::Month => month_page_name(date.year, date.month),
            DateUnit::Year => year_page_name(date.year),
        };
        self.send(Event::GoToPage {
            path,
            open_in: client_core::keys::OpenIn::Here,
        });
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
        // Emacs has this text already: the render below sends back only its EmacsText
        if let Some(sent) = self.sent.rows.iter_mut().find(|sent| sent.row.id == id) {
            sent.row.body = body.clone();
        }
        let now_ms = self.host.now().now_ms;
        self.send(Event::BodyChanged {
            item_id: id,
            body,
            now_ms,
        });
    }

    fn typed(&mut self, id: String, cursor: usize) {
        let Some(body) = self.body_of(&id) else {
            return;
        };
        let now_ms = self.host.now().now_ms;
        // The core closes the opening just typed (ADR 057). Emacs does not have the closed
        // text yet: the render sends it
        self.send(Event::Typed {
            item_id: id.clone(),
            body: body.clone(),
            cursor,
            now_ms,
        });
        // The render is done: the cursor goes back after it
        if self.body_of(&id).is_some_and(|after| after != body) {
            self.host.emit(Outbound::Focus {
                ack: self.ack,
                id,
                cursor: Some(cursor),
            });
        }
    }

    /// The Markdown of the item as the core has it.
    fn body_of(&self, id: &str) -> Option<String> {
        self.core
            .view()
            .outline
            .rows
            .iter()
            .find(|r| r.item.id == id)
            .map(|r| r.item.body.clone())
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
        // Emacs has one question at a time, and answers it once
        let (asked, mut request) = self.asked.take().expect("the question waits");
        assert_eq!(req, asked, "Emacs answers the question it has");
        // Before the answer: the next question reaches Emacs with what the answer changes
        self.ask_next();
        self.resolve(&mut request, UiOutput::Bool(yes));
    }

    /// Opens the URL, and the core keeps it in its history (ADR 035).
    fn open(&mut self, url: String) {
        let today = self.today();
        self.send(Event::Open { url, today });
    }

    /// The local date, as `YYYY-MM-DD`.
    fn today(&self) -> String {
        self.host.now().local.day_name()
    }

    fn send(&mut self, event: Event) {
        let effects = self.core.process_event(event);
        self.process(effects);
    }

    fn resolve<Op: Operation>(&mut self, request: &mut Request<Op>, output: Op::Output) {
        // Each request is kept until its answer and no longer, so the core is waiting for it
        let effects = self
            .core
            .resolve(request, output)
            .expect("the core waits for the answer");
        self.process(effects);
    }

    fn process(&mut self, effects: Vec<Effect>) {
        let mut render = false;
        let mut focused = Vec::new();
        let mut reads = Vec::new();
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
                Effect::Ui(request) if request.operation == UiOp::ReadClipboardText => {
                    reads.push(request);
                }
                Effect::Ui(request) => {
                    if let Some(request) = self.on_ui_op(request) {
                        focused.push(request);
                    }
                }
                // A notification: the core does not wait for it
                // The core keeps whether it replaces the page on screen
                Effect::Navigate(request) => {
                    self.navigations.push_back(request.operation.url.clone());
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
        for mut request in reads {
            let text = self.clipboard.clone();
            self.resolve(&mut request, UiOutput::Text(text));
        }
        while let Some(url) = self.navigations.pop_front() {
            self.open(url);
        }
    }

    fn on_timer_op(&mut self, request: Request<TimerOp>) {
        match request.operation {
            TimerOp::Start { id, ms } => {
                if ms == CHECK_MS {
                    self.clock_watch = Some(id);
                }
                self.timers.start(id, request);
                self.host.start_timer(id, ms);
            }
            TimerOp::Cancel { id } => {
                if let Some(mut running) = self.timers.end(id) {
                    self.resolve(&mut running, TimerOutput::Cancelled);
                }
            }
        }
    }

    /// Performs a screen operation. A focus request comes back, to be resolved after the render.
    fn on_ui_op(&mut self, request: Request<UiOp>) -> Option<Request<UiOp>> {
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
            UiOp::CopyMarkdown { markdown: text, .. } => {
                self.host.emit(Outbound::Copy { text });
            }
            UiOp::Confirm(message) | UiOp::ConfirmDestructive(message) => {
                self.ask(message, request);
            }
            // There is nothing to do these with here: the atlas map is a part of the web shell.
            // A new page from the header and a page in another window start from events that
            // Emacs does not send. The clipboard is read in `process`. A failed copy leaves the
            // kill ring as it was
            // The core looks at the clock for a new day (#1031)
            UiOp::LocalNow => {
                let Now { local, now_ms } = self.host.now();
                let mut request = request;
                self.resolve(&mut request, UiOutput::Now { local, now_ms });
            }
            UiOp::OpenInBrowser { address } => self.host.emit(Outbound::Browse { address }),
            // Emacs sends no keys with a click, so the core asks for no new view (ADR 083)
            UiOp::OpenNewView { .. }
            | UiOp::SaveAtlasDirs(_)
            | UiOp::CopyFailed
            | UiOp::Prompt(_)
            | UiOp::ReadClipboardText => {}
        }
        None
    }

    fn ask(&mut self, message: String, request: Request<UiOp>) {
        if self.asks.push((message, request)) {
            self.ask_next();
        }
    }

    /// Gives Emacs the question whose turn it is, if one waits.
    fn ask_next(&mut self) {
        self.asked = self.asks.take_next().map(|(message, request)| {
            self.next_ask += 1;
            self.after_render.push(Outbound::Ask {
                req: self.next_ask,
                message,
            });
            (self.next_ask, request)
        });
    }

    /// Tells Emacs what changed in the view. What Emacs is told of a panel is made only when
    /// the panel changed (ADR 078).
    fn tell_view(&mut self) {
        let view = self.core.view();
        self.relay_holding = view.outline.key_relay_holding;
        let changed = self.panels.changes(&view);
        let any = |is: fn(&Panel) -> bool| changed.iter().any(is);
        self.tell_rows(&view);
        if any(|panel| matches!(panel, Panel::Outline(_))) {
            self.tell_focus(&view);
        }
        if any(|panel| matches!(panel, Panel::Screen(_) | Panel::Config(_))) {
            self.tell_page_path(&view);
            self.tell_index(&view);
            self.tell_properties(&view);
            self.tell_backlinks(&view);
        }
        if any(|panel| matches!(panel, Panel::Connection(_))) {
            self.tell_connection(&view);
        }
        if any(|panel| matches!(panel, Panel::Search(_))) {
            self.tell_search(&view.search);
        }
        if any(|panel| {
            matches!(
                panel,
                Panel::IndexPages(_) | Panel::Headings(_) | Panel::RecentPages(_)
            )
        }) {
            self.tell_recent(&view);
            self.tell_sidebar(&view);
        }
    }

    fn tell_sidebar(&mut self, view: &ViewModel<SharedText>) {
        let sidebar = SidebarView::of(&view.index_pages, &view.headings, &view.recent_pages);
        if sidebar != self.sent.sidebar {
            self.sent.sidebar.clone_from(&sidebar);
            self.host.emit(Outbound::Sidebar(sidebar));
        }
    }

    fn tell_rows(&mut self, view: &ViewModel<SharedText>) {
        let rows: Vec<SentRow> = view.outline.rows.iter().map(SentRow::of).collect();
        let page_id = view.index_pages.current_page_id.clone();
        let changes = list_changes(&self.sent.rows, &rows, |sent| &sent.row.id);
        if changes.is_empty() && page_id == self.sent.page_id {
            return;
        }
        self.sent.rows = rows;
        self.sent.page_id.clone_from(&page_id);
        self.host.emit(Outbound::Rows {
            ack: self.ack,
            page_id,
            changes: changes
                .into_iter()
                .map(|change| RowChange::from(change.map(SentRow::into_view)))
                .collect(),
        });
    }

    fn tell_focus(&mut self, view: &ViewModel<SharedText>) {
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

    fn tell_page_path(&mut self, view: &ViewModel<SharedText>) {
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

    fn tell_index(&mut self, view: &ViewModel<SharedText>) {
        let index = match &view.screen {
            ScreenView::Page(Some(page)) if page.is_index && !page.is_root_index => {
                let folder = page.page.dir_path.trim_end_matches('/').to_string();
                let demote_name = page.can_demote.then(|| page.demote_name.clone());
                (Some(folder), demote_name)
            }
            _ => (None, None),
        };
        if index != self.sent.index {
            self.sent.index.clone_from(&index);
            let (folder, demote_name) = index;
            self.host.emit(Outbound::Index {
                folder,
                demote_name,
            });
        }
    }

    fn tell_properties(&mut self, view: &ViewModel<SharedText>) {
        let page = match &view.screen {
            ScreenView::Page(Some(page)) => Some(page),
            _ => None,
        };
        let rows: Option<Vec<PropertyRow>> = page.map(|page| {
            page.properties
                .iter()
                .map(|p| PropertyRow::of(p, label(view, &p.key)))
                .collect()
        });
        let free_keys: Vec<FreeKey> = page
            .iter()
            .flat_map(|page| &page.free_keys)
            .map(|free| FreeKey {
                key: free.key.clone(),
                label: label(view, &free.key),
            })
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

    fn tell_connection(&mut self, view: &ViewModel<SharedText>) {
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
                self.host.emit(Outbound::Search {
                    results: results.iter().map(SearchEntry::of).collect(),
                    searching,
                });
            }
        }
    }

    fn tell_recent(&mut self, view: &ViewModel<SharedText>) {
        let paths: Vec<String> = view
            .recent_pages
            .pages
            .iter()
            .map(|p| p.path.clone())
            .collect();
        if paths != self.sent.recent {
            self.sent.recent.clone_from(&paths);
            self.host.emit(Outbound::Recent { paths });
        }
    }

    fn tell_backlinks(&mut self, view: &ViewModel<SharedText>) {
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
        // The link goes in at the cursor the insertion started with: a key typed in the meantime
        // would move the text under it
        let holding = self.relay_holding || !self.image_calls.is_empty();
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

/// The names of the pages a search found. None when the search failed: the core writes the
/// error to its log.
fn page_titles(result: &ApiResult<ApiOutput>) -> Vec<String> {
    match result {
        Ok(ApiOutput::SearchResults(results)) => {
            results.iter().map(|r| r.page_title.clone()).collect()
        }
        _ => Vec::new(),
    }
}

/// The keys whose value is a date and a time. Only the page's own dates cannot be removed, and
/// they are dates without a definition.
fn datetime_keys<'a>(
    view: &ViewModel<SharedText>,
    rows: impl Iterator<Item = &'a PropertyRow>,
    free_keys: &[FreeKey],
) -> Vec<String> {
    let is_datetime = |key: &str| property_type(view, key) == "datetime";
    rows.filter(|row| !row.removable || is_datetime(&row.key))
        .map(|row| row.key.clone())
        .chain(
            free_keys
                .iter()
                .filter(|free| is_datetime(&free.key))
                .map(|free| free.key.clone()),
        )
        .collect()
}

/// What Emacs shows for the key: the body of its name and its type, so that two keys that
/// share a body are told apart (ADR 047). A system key keeps its whole name.
fn label(view: &ViewModel<SharedText>, key: &str) -> String {
    let name = bare_name(key, &view.config.property_definitions);
    format!("{name} ({})", property_type(view, key))
}

/// The type to send a value of the key with: the one the core gives the row of the page, or
/// the one of the definition for a key the page does not have yet.
fn property_type(view: &ViewModel<SharedText>, key: &str) -> String {
    let row = match &view.screen {
        ScreenView::Page(Some(page)) => page.properties.iter().find(|p| p.key == key),
        _ => None,
    };
    row.map_or_else(
        || value_type_of(key, false, &view.config.property_definitions),
        |row| row.value_type.clone(),
    )
}
