//! The Crux app: the model, the events the shell sends, and `update`.
//!
//! Operations that wait for the server are written as async commands. A command cannot
//! touch the model, so it sends [`Event::Continue`] with a closure that does; the closure
//! runs in `update` before the next effect reaches the shell. This keeps the ported
//! service code close to the Angular original, where state changes and `await`s interleave.

use crate::effects::{Api, Effect, SseEvent};
use crate::model::*;
use crate::ops;
use crate::outline::{KeyRelay, OutlineState, Selection, UndoStacks};
use crate::screens;
use crate::util::filename::JournalType;
use crux_core::command::CommandContext;
use crux_core::{render, App, Command};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fmt;
use std::future::Future;

pub type Cmd = Command<Effect, Event>;
pub type Ctx = CommandContext<Effect, Event>;

/// A continuation: the rest of an operation once the server has answered.
pub struct Thunk {
    pub label: &'static str,
    pub run: Box<dyn FnOnce(&mut Model) -> Cmd + Send>,
}

impl fmt::Debug for Thunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Continue({})", self.label)
    }
}

/// What the editor reports about the `[[wiki link]]` being typed: the text inside and where
/// the caret is on the screen.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WikiLinkContext {
    pub query: String,
    pub x: f64,
    pub y: f64,
}

/// Keys the shell forwards while the outline is in selection mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionKey {
    Escape,
    Delete,
    MoveUp,
    MoveDown,
    Indent,
    Outdent,
    ExtendUp,
    ExtendDown,
    Copy,
    Cut,
    Undo,
    Redo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchKey {
    Down,
    Up,
    Enter,
    Escape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalNav {
    PrevDay,
    NextDay,
    PrevWeek,
    NextWeek,
}

/// The URL the shell is showing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    /// `/pages/<name>`, `/docs/<name>`, `/journals/<name>` or `/today` (name `today`).
    Page { name: String, fragment: Option<String> },
    Settings,
}

#[derive(Debug)]
pub enum Event {
    /// The shell is up: connect, load the settings and the sidebar.
    Start,
    /// The URL changed; `today` is the local date as `YYYY-MM-DD`.
    Route { route: Route, today: String },
    Continue(Thunk),

    // ---- the outline ----
    EditRequested { item_id: String, cursor: usize },
    BodyChanged { item_id: String, body: String, now_ms: u64 },
    EditFinished { item_id: String },
    EnterPressed { item_id: String, cursor: usize, now_ms: u64 },
    BackspaceAtStart { item_id: String },
    DeleteAtEnd { item_id: String },
    NavigateUp { item_id: String, cursor: usize },
    NavigateDown { item_id: String, cursor: usize },
    Indent { item_id: String },
    Outdent { item_id: String },
    MoveUp { item_id: String },
    MoveDown { item_id: String },
    TaskCycle { item_id: String },
    CollapseToggle { item_id: String },
    /// Shift+Wheel: a tenth more or less zoom.
    ZoomStep { item_id: String, up: bool },
    ResetZoom { item_id: String },
    PasteText { item_id: String, text: String, html: String, cursor: usize },
    PasteImage { item_id: String, file: u64, cursor: usize },
    DragBeyondBounds { item_id: String },
    ItemMouseEnter { item_id: String },
    /// The mouse button was released during a drag selection.
    SelectionMouseUp,
    /// Typing inside `[[ ]]`; None closes the popup.
    WikiQuery { item_id: String, context: Option<WikiLinkContext> },
    /// ArrowUp, ArrowDown or Enter while the popup is open.
    WikiKey { key: String },
    WikiPick { title: String },
    OpenWikiLink { text: String },
    ShiftSelect { item_id: String },
    Undo,
    Redo,
    PasteAsTransclusion { item_id: String },
    RemoveTransclusion { item_id: String },
    SendToArchive { item_id: String },
    /// A key pressed while the relay holds keys for the next editor.
    KeyHeld(DeferredKey),
    SelectionKey(SelectionKey),
    SelectionClickOutside,
    OutlineScrolled { scrolled_from_top: bool, top_visible_index: usize, rendered_start: usize, rendered_end: usize },
    ScrollToItemRequested { item_id: String },

    // ---- pages and layout ----
    PageTitleSave { dir_path: String, file_name: String },
    DemoteIndex,
    /// The "+" button; the search box text is used as the title when present.
    CreateNewPage { search_query: String },
    DeleteCurrentPage,
    NavigateToToday,
    NavigateToRoot,
    /// Opens a page by its path, from the sidebar, the calendar or a link.
    GoToPage { path: String, new_tab: bool, new_window: bool },
    JournalNav(JournalNav),
    ToggleSidebarSection { section: String },

    // ---- search ----
    SearchQueryChanged(String),
    SearchKey(SearchKey),
    SearchResultClicked { index: usize },
    SearchCreateClicked,
    SearchDropdown(bool),

    // ---- settings ----
    SetWeekNumbering(WeekNumbering),
    SetLogicalOutdent(bool),
    BrowseFolder,

    // ---- extensions and links ----
    ExtensionCreateItem {
        source: u64,
        request_id: Option<String>,
        page_id: String,
        position: Option<String>,
        after_item_id: Option<String>,
        level: i32,
        body: String,
    },
    LaunchUrl { url: String },

    // ---- from the shell's streams ----
    Sse(SseEvent),
}

/// Builds a continuation event.
pub fn cont(label: &'static str, run: impl FnOnce(&mut Model) -> Cmd + Send + 'static) -> Event {
    Event::Continue(Thunk { label, run: Box::new(run) })
}

/// Sends a continuation from inside an async command.
pub fn apply(api: &Api<Event>, label: &'static str, run: impl FnOnce(&mut Model) -> Cmd + Send + 'static) {
    api.send_event(cont(label, run));
}

pub fn done() -> Cmd {
    Command::done()
}

/// Runs an async operation with an [`Api`] handle.
pub fn run<F, Fut>(f: F) -> Cmd
where
    F: FnOnce(Api<Event>) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    Command::new(move |ctx| f(Api::new(ctx)))
}

pub fn all(cmds: impl IntoIterator<Item = Cmd>) -> Cmd {
    Command::all(cmds)
}

/// Logs an error the user does not need to see, like `console.error` in the Angular app.
pub fn log_error(message: impl fmt::Display) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = message;
    }
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("[peta-core] {message}");
}

// ---- Model ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionState {
    Connecting,
    Connected,
    #[default]
    Disconnected,
    Error,
}

impl ConnectionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectionState::Connecting => "connecting",
            ConnectionState::Connected => "connected",
            ConnectionState::Disconnected => "disconnected",
            ConnectionState::Error => "error",
        }
    }
}

/// Typed text waiting for the auto-save.
#[derive(Debug, Clone)]
pub struct PendingSave {
    pub body: String,
    pub timer_id: u64,
}

/// The life of an edit, see `ops::edit`.
#[derive(Debug, Default)]
pub struct EditState {
    /// Shown item IDs whose body was typed into and not yet saved.
    pub dirty_item_ids: HashSet<String>,
    /// Handlers that save the edited item and choose the next edited item themselves;
    /// while above zero the blur of the editor is ignored.
    pub edit_handovers: u32,
    /// Saved item id → typed text waiting for its timer.
    pub pending_saves: HashMap<String, PendingSave>,
}

#[derive(Debug, Default)]
pub struct JournalState {
    pub page_name: String,
    pub current_date: String,
    pub journal_type: Option<JournalType>,
    /// Date of the top visible item; shown in the journal header only.
    pub top_visible_date: String,
    pub page: Option<Page>,
    /// Names of the journals already appended to the outline.
    pub loaded_dates: HashSet<String>,
    /// True while older journals are being fetched; a second request is dropped.
    pub loading: bool,
}

#[derive(Debug, Default)]
pub struct PageState {
    pub page_name: String,
    pub focus_item_id: Option<String>,
    pub page: Option<Page>,
    pub can_demote_index: bool,
}

#[derive(Debug, Default)]
pub enum Screen {
    #[default]
    None,
    Journal(JournalState),
    Page(PageState),
    Settings,
}

#[derive(Debug, Default)]
pub struct SearchState {
    pub query: String,
    pub page_results: Vec<SearchResult>,
    pub item_results: Vec<SearchResult>,
    pub show_dropdown: bool,
    pub show_create_button: bool,
    pub selected_index: Option<usize>,
    pub timer_id: u64,
}

#[derive(Debug, Default)]
pub struct WikiState {
    pub item_id: Option<String>,
    pub query: String,
    pub x: f64,
    pub y: f64,
    pub suggestions: Vec<String>,
    pub index: usize,
    pub timer_id: u64,
}

#[derive(Debug, Default)]
pub struct ConfigState {
    pub markdown_dir: String,
    pub week_numbering: WeekNumbering,
    pub logical_outdent: bool,
    pub property_definitions: BTreeMap<String, String>,
}

#[derive(Debug, Default)]
pub struct SidebarState {
    pub index_header: String,
    pub index_pages: Vec<Page>,
    pub collapsed_sections: HashSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveDirection {
    Up,
    Down,
}

pub struct Model {
    pub state: OutlineState,
    pub undo: UndoStacks,
    pub selection: Selection,
    pub key_relay: KeyRelay,
    pub edits: EditState,
    /// Pages changed on disk while the user was editing.
    pub file_reload_pending: HashSet<String>,
    /// True while a reload of the pending pages waits for the auto-save to flush.
    pub file_reload_scheduled: bool,
    /// Moves run one after another; see `ops::moves`.
    pub move_in_progress: bool,
    pub move_queue: VecDeque<(String, MoveDirection)>,
    pub screen: Screen,
    pub current_page: Option<Page>,
    pub recent_pages: Vec<Page>,
    pub sidebar: SidebarState,
    pub search: SearchState,
    pub wiki: WikiState,
    pub config: ConfigState,
    pub connection: ConnectionState,
    pub sse_reconnect_attempts: u32,
    pub scrolled_from_top: bool,
    /// The flat and tree markdown of the last copy, so a paste back keeps the structure.
    pub clipboard: Option<(String, String)>,
    /// The local date the shell last reported, `YYYY-MM-DD`.
    pub today: String,
    pub hmtid: hmtid::Generator,
    /// The last wall-clock time the shell reported, in milliseconds.
    pub clock_ms: u64,
    pub next_timer_id: u64,
    pub extension_functions_loaded: bool,
}

impl Default for Model {
    fn default() -> Self {
        Model {
            state: OutlineState::default(),
            undo: UndoStacks::default(),
            selection: Selection::default(),
            key_relay: KeyRelay::default(),
            edits: EditState::default(),
            file_reload_pending: HashSet::new(),
            file_reload_scheduled: false,
            move_in_progress: false,
            move_queue: VecDeque::new(),
            screen: Screen::None,
            current_page: None,
            recent_pages: Vec::new(),
            sidebar: SidebarState::default(),
            search: SearchState::default(),
            wiki: WikiState::default(),
            config: ConfigState::default(),
            connection: ConnectionState::Disconnected,
            sse_reconnect_attempts: 0,
            scrolled_from_top: false,
            clipboard: None,
            today: String::new(),
            hmtid: hmtid::Generator::new('-', true),
            clock_ms: 0,
            next_timer_id: 1,
            extension_functions_loaded: false,
        }
    }
}

impl Model {
    pub fn next_timer_id(&mut self) -> u64 {
        let id = self.next_timer_id;
        self.next_timer_id += 1;
        id
    }

    /// The page the outline shows: the journal's own page or the doc page.
    pub fn shown_page_id(&self) -> Option<String> {
        match &self.screen {
            Screen::Journal(j) => j.page.as_ref().map(|p| p.id.clone()),
            Screen::Page(p) => p.page.as_ref().map(|p| p.id.clone()),
            _ => None,
        }
    }

    /// The item as the state has it now, as shown. Handlers that read the body or level
    /// start from this, because what the shell rendered lags behind fast typing.
    pub fn current_item(&self, item_id: &str) -> Option<ItemRef> {
        self.state.get_visible_item(item_id).or_else(|| self.state.get_item_by_id(item_id))
    }

    /// A fresh HMTID for an item created on the client.
    pub fn new_item_id(&self, now_ms: u64) -> String {
        self.hmtid.generate_at(now_ms).as_str().to_string()
    }
}

// ---- The app ----

#[derive(Default)]
pub struct PetaApp;

impl App for PetaApp {
    type Event = Event;
    type Model = Model;
    type ViewModel = crate::view::ViewModel;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Cmd {
        let cmd = dispatch(event, model);
        let flush = ops::file_reload::schedule_pending_flush(model);
        Command::all([cmd, flush, render::render()])
    }

    fn view(&self, model: &Model) -> Self::ViewModel {
        crate::view::build(model)
    }
}

fn dispatch(event: Event, model: &mut Model) -> Cmd {
    use Event::*;
    match event {
        Start => screens::start::start(model),
        Route { route, today } => screens::route::route(model, route, today),
        Continue(thunk) => (thunk.run)(model),

        EditRequested { item_id, cursor } => ops::outline_events::edit_requested(model, &item_id, cursor),
        BodyChanged { item_id, body, now_ms } => {
            model.clock_ms = now_ms;
            ops::outline_events::body_changed(model, &item_id, body)
        }
        EditFinished { item_id } => ops::edit::finish_edit(model, &item_id),
        EnterPressed { item_id, cursor, now_ms } => {
            model.clock_ms = now_ms;
            ops::split::handle_enter(model, &item_id, cursor)
        }
        BackspaceAtStart { item_id } => ops::merge::merge_into_previous(model, &item_id),
        DeleteAtEnd { item_id } => ops::merge::merge_next_into(model, &item_id),
        NavigateUp { item_id, cursor } => ops::outline_events::navigate(model, &item_id, cursor, -1),
        NavigateDown { item_id, cursor } => ops::outline_events::navigate(model, &item_id, cursor, 1),
        Indent { item_id } => ops::level::indent(model, &item_id),
        Outdent { item_id } => ops::level::outdent(model, &item_id),
        MoveUp { item_id } => ops::moves::enqueue_move(model, &item_id, MoveDirection::Up),
        MoveDown { item_id } => ops::moves::enqueue_move(model, &item_id, MoveDirection::Down),
        TaskCycle { item_id } => ops::action::cycle_task(model, &item_id),
        CollapseToggle { item_id } => ops::action::toggle_collapsed(model, &item_id),
        ZoomStep { item_id, up } => ops::action::zoom_step(model, &item_id, up),
        ResetZoom { item_id } => ops::action::set_zoom(model, &item_id, 1.0),
        PasteText { item_id, text, html, cursor } => ops::paste::paste_text(model, &item_id, text, html, cursor),
        PasteImage { item_id, file, cursor } => ops::paste::paste_image(model, &item_id, file, cursor),
        DragBeyondBounds { item_id } => {
            model.selection.start_drag(&mut model.state, &item_id);
            done()
        }
        ItemMouseEnter { item_id } => {
            model.selection.extend_drag_to(&mut model.state, &item_id);
            done()
        }
        SelectionMouseUp => {
            model.selection.end_drag();
            done()
        }
        WikiQuery { item_id, context } => ops::wiki::query(model, &item_id, context),
        WikiKey { key } => ops::wiki::key_nav(model, &key),
        WikiPick { title } => ops::wiki::pick(model, &title),
        OpenWikiLink { text } => screens::navigate::to_page(&text, false, false, false, None),
        ShiftSelect { item_id } => {
            model.selection.start_from_editor(&mut model.state, &item_id);
            done()
        }
        Undo => ops::undo_replay::perform(model, ops::undo_replay::Direction::Undo),
        Redo => ops::undo_replay::perform(model, ops::undo_replay::Direction::Redo),
        PasteAsTransclusion { item_id } => ops::action::paste_as_transclusion(model, &item_id),
        RemoveTransclusion { item_id } => ops::action::remove_transclusion(model, &item_id),
        SendToArchive { item_id } => ops::action::archive(model, &item_id),
        KeyHeld(key) => {
            model.key_relay.hold(key);
            done()
        }
        SelectionKey(key) => ops::outline_events::selection_key(model, key),
        SelectionClickOutside => {
            if model.state.item_selection_mode() && !model.selection.is_dragging() {
                model.selection.exit(&mut model.state);
            }
            done()
        }
        OutlineScrolled { scrolled_from_top, top_visible_index, rendered_start, rendered_end } => {
            screens::journal::outline_scrolled(model, scrolled_from_top, top_visible_index, rendered_start, rendered_end)
        }
        ScrollToItemRequested { item_id } => {
            Command::notify_shell(crate::effects::UiOp::ScrollToItem { item_id, smooth: true }).into()
        }

        PageTitleSave { dir_path, file_name } => screens::page::save_title(model, dir_path, file_name),
        DemoteIndex => screens::page::demote_index(model),
        CreateNewPage { search_query } => screens::layout::create_new_page(model, search_query),
        DeleteCurrentPage => screens::layout::delete_current_page(model),
        NavigateToToday => Command::notify_shell(crate::effects::NavigateOp { url: "/today".into(), replace: false }).into(),
        NavigateToRoot => screens::navigate::to_page("index", false, false, false, None),
        GoToPage { path, new_tab, new_window } => screens::navigate::to_page(&path, new_tab, new_window, false, None),
        JournalNav(nav) => screens::journal::nav(model, nav),
        ToggleSidebarSection { section } => {
            if !model.sidebar.collapsed_sections.remove(&section) {
                model.sidebar.collapsed_sections.insert(section);
            }
            done()
        }

        SearchQueryChanged(q) => screens::search::query_changed(model, q),
        SearchKey(key) => screens::search::key(model, key),
        SearchResultClicked { index } => screens::search::result_clicked(model, index),
        SearchCreateClicked => screens::search::create_page(model),
        SearchDropdown(show) => {
            model.search.show_dropdown = show;
            done()
        }

        SetWeekNumbering(value) => screens::settings::set_week_numbering(model, value),
        SetLogicalOutdent(value) => screens::settings::set_logical_outdent(model, value),
        BrowseFolder => screens::settings::browse_folder(model),

        ExtensionCreateItem { source, request_id, page_id, position, after_item_id, level, body } => {
            screens::extensions::create_item(model, source, request_id, page_id, position, after_item_id, level, body)
        }
        LaunchUrl { url } => run(move |api| async move {
            if !api.launch_url(&url).await.unwrap_or(false) {
                log_error(format!("Failed to launch app with URL: {url}"));
            }
        }),

        Sse(event) => screens::sse::on_event(model, event),
    }
}
