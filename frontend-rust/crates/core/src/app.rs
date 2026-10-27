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
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
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
    Page {
        name: String,
        fragment: Option<String>,
    },
    Settings,
}

#[derive(Debug)]
pub enum Event {
    /// The shell is up: connect, load the settings and the sidebar.
    Start,
    /// The URL changed; `today` is the local date as `YYYY-MM-DD`.
    Route {
        route: Route,
        today: String,
    },
    Continue(Thunk),

    // ---- the outline ----
    EditRequested {
        item_id: String,
        cursor: usize,
    },
    BodyChanged {
        item_id: String,
        body: String,
        now_ms: u64,
    },
    EditFinished {
        item_id: String,
    },
    EnterPressed {
        item_id: String,
        cursor: usize,
        now_ms: u64,
    },
    BackspaceAtStart {
        item_id: String,
    },
    DeleteAtEnd {
        item_id: String,
    },
    NavigateUp {
        item_id: String,
        cursor: usize,
    },
    NavigateDown {
        item_id: String,
        cursor: usize,
    },
    Indent {
        item_id: String,
    },
    Outdent {
        item_id: String,
    },
    MoveUp {
        item_id: String,
    },
    MoveDown {
        item_id: String,
    },
    TaskCycle {
        item_id: String,
    },
    CollapseToggle {
        item_id: String,
    },
    /// Shift+Wheel: a tenth more or less zoom.
    ZoomStep {
        item_id: String,
        up: bool,
    },
    ResetZoom {
        item_id: String,
    },
    PasteText {
        item_id: String,
        text: String,
        html: String,
        cursor: usize,
    },
    PasteImage {
        item_id: String,
        file: u64,
        cursor: usize,
    },
    DragBeyondBounds {
        item_id: String,
    },
    ItemMouseEnter {
        item_id: String,
    },
    /// The mouse button was released during a drag selection.
    SelectionMouseUp,
    /// Typing inside `[[ ]]`; None closes the popup.
    WikiQuery {
        item_id: String,
        context: Option<WikiLinkContext>,
    },
    /// ArrowUp, ArrowDown or Enter while the popup is open.
    WikiKey {
        key: String,
    },
    WikiPick {
        title: String,
    },
    OpenWikiLink {
        text: String,
    },
    ShiftSelect {
        item_id: String,
    },
    Undo,
    Redo,
    PasteAsTransclusion {
        item_id: String,
    },
    RemoveTransclusion {
        item_id: String,
    },
    SendToStash {
        item_id: String,
    },
    /// A key pressed while the relay holds keys for the next editor.
    KeyHeld(DeferredKey),
    SelectionKey(SelectionKey),
    SelectionClickOutside,
    OutlineScrolled {
        scrolled_from_top: bool,
        top_visible_index: usize,
        rendered_start: usize,
        rendered_end: usize,
    },
    ScrollToItemRequested {
        item_id: String,
    },

    // ---- pages and layout ----
    PageTitleSave {
        dir_path: String,
        file_name: String,
    },
    DemoteIndex,
    /// The map button: switches between the index page and the atlas page of a directory.
    ToggleAtlas,
    /// The up button of an atlas page: opens the atlas of the directory above.
    AtlasUp,
    /// A card dropped on the atlas canvas: the page's card is placed with its center there.
    PlaceAtlasCard {
        page_id: String,
        x: f64,
        y: f64,
    },
    /// The selected cards scaled with Ctrl and the wheel: the page's card takes the scale.
    ScaleAtlasCard {
        page_id: String,
        scale: f64,
    },
    /// The group menu's "Edit Title": the group's title lies along a side of its outline.
    /// An empty title takes it away.
    SetAtlasGroupTitle {
        group_key: String,
        side: String,
        title: String,
    },
    /// The canvas menu's "Add Callout": a callout with its character centered at the point
    /// and its bubble on a side of it. An empty text adds nothing.
    AddAtlasCallout {
        x: f64,
        y: f64,
        side: String,
        text: String,
    },
    /// A callout dragged to a new place: the center of its character
    MoveAtlasCallout {
        key: String,
        x: f64,
        y: f64,
    },
    /// The selected callouts scaled with Ctrl and the wheel
    ScaleAtlasCallout {
        key: String,
        scale: f64,
    },
    /// The canvas menu's "Add Link": a link card with its center at the point, linking to the
    /// target page
    AddAtlasLink {
        x: f64,
        y: f64,
        target: String,
    },
    /// A link card dragged to a new place: its center
    MoveAtlasLink {
        item_id: String,
        x: f64,
        y: f64,
    },
    /// The selected link cards scaled with Ctrl and the wheel
    ScaleAtlasLink {
        item_id: String,
        scale: f64,
    },
    /// The link card menu's "Edit Link": the card links to another page
    SetAtlasLink {
        item_id: String,
        target: String,
    },
    /// The link card menu's "Remove Link": the item of the link is deleted
    RemoveAtlasLink {
        item_id: String,
    },
    /// The text typed in the dialog of a link, to find page names for it
    SearchAtlasLinks {
        query: String,
    },
    /// The callout menu's "Edit Callout". An empty text takes the callout away.
    SetAtlasCallout {
        key: String,
        side: String,
        text: String,
    },
    /// The card menu's "Remove from Atlas": the page's card goes back to the unplaced list.
    RemoveAtlasCard {
        page_id: String,
    },
    /// Cards dropped on another card with Ctrl held: they join the group of that card.
    GroupAtlasCards {
        page_ids: Vec<String>,
        target_id: String,
    },
    /// Cards dragged out of the outline of their group with Ctrl held: they leave it.
    UngroupAtlasCards {
        page_ids: Vec<String>,
    },
    /// The page menu: show the property rows of the doc page even while it has none.
    EditPageProperties,
    SetPageProperty {
        key: String,
        value_type: String,
        value: String,
    },
    DeletePageProperty {
        key: String,
    },
    /// The "+" button; the search box text is used as the title when present.
    CreateNewPage {
        search_query: String,
    },
    DeleteCurrentPage,
    NavigateToToday,
    /// The Home button: the page set in the settings, or today
    NavigateHome,
    NavigateToRoot,
    /// Opens the index page of the folder of the tag pages.
    NavigateToTags,
    /// Opens a page by its path, from the sidebar, the calendar or a link.
    GoToPage {
        path: String,
        new_tab: bool,
        new_window: bool,
    },
    JournalNav(JournalNav),

    // ---- search ----
    SearchQueryChanged(String),
    SearchKey(SearchKey),
    SearchResultClicked {
        index: usize,
    },
    SearchDropdown(bool),

    // ---- sidebar ----
    /// The shell restored the per-directory sorts it had stored.
    IndexSortsRestored(HashMap<String, IndexSort>),
    /// The folders last seen as their atlas, as the browser stored them.
    AtlasDirsRestored(Vec<String>),
    /// The user chose a sort for the directory the index list shows.
    SetIndexSort {
        dir: String,
        sort: IndexSort,
    },

    // ---- settings ----
    SetWeekNumbering(WeekNumbering),
    SetLogicalOutdent(bool),
    SetTagsDir(String),
    SetHomeLink(String),
    BrowseFolder,
    /// The settings form: define a user key of the given type.
    AddPropertyDefinition {
        key: String,
        value_type: String,
    },
    DeletePropertyDefinition {
        key: String,
    },

    // ---- extensions and links ----
    ExtensionCreateItem {
        source: u64,
        request_id: Option<String>,
        page_id: String,
        position: Option<String>,
        after_item_id: Option<String>,
        level: i32,
        body: String,
        /// The id of the item the new item transcludes; None or empty for none.
        transclude: Option<String>,
    },
    /// A `Confirm` message from an extension: asks the user and answers with `ConfirmResult`.
    ExtensionConfirm {
        source: u64,
        request_id: Option<String>,
        message: String,
    },
    LaunchUrl {
        url: String,
    },

    // ---- from the shell's streams ----
    Sse(SseEvent),
}

/// Builds a continuation event.
pub fn cont(label: &'static str, run: impl FnOnce(&mut Model) -> Cmd + Send + 'static) -> Event {
    Event::Continue(Thunk {
        label,
        run: Box::new(run),
    })
}

/// Sends a continuation from inside an async command.
pub fn apply(
    api: &Api<Event>,
    label: &'static str,
    run: impl FnOnce(&mut Model) -> Cmd + Send + 'static,
) {
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
    /// The item the URL fragment names, scrolled to once the journal loads.
    pub focus_item_id: Option<String>,
}

#[derive(Debug, Default)]
pub struct PageState {
    pub page_name: String,
    pub focus_item_id: Option<String>,
    pub page: Option<Page>,
    pub can_demote_index: bool,
    /// True once the menu opened the property rows; a page with properties shows them anyway.
    pub properties_open: bool,
    pub backlinks: Vec<BacklinkEntry>,
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

/// The page names offered in the dialog of an atlas link, and the timer of the pause before
/// they are searched.
#[derive(Debug, Default)]
pub struct AtlasLinkSearch {
    pub suggestions: Vec<String>,
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
    /// Directory of the tag pages relative to the pages root, `tags/` style; empty for the root.
    pub tags_dir: String,
    /// The page the Home button opens, as the target of a wiki link; empty for today.
    pub home_link: String,
    pub property_definitions: Vec<PropertyDefinition>,
    /// Why the last definition was refused, shown under the settings form until the next try.
    pub definition_error: Option<String>,
}

#[derive(Debug, Default)]
pub struct SidebarState {
    pub index_header: String,
    pub index_pages: Vec<Page>,
    /// The sort of each directory's index list, by its header (`dir/` or `(root)/`).
    pub index_sorts: HashMap<String, IndexSort>,
    /// The folders last seen as their atlas page, by dir path (empty for the root). A
    /// folder opens as it was last seen.
    pub atlas_dirs: BTreeSet<String>,
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
    pub atlas_link_search: AtlasLinkSearch,
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
            atlas_link_search: AtlasLinkSearch::default(),
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
        self.state
            .get_visible_item(item_id)
            .or_else(|| self.state.get_item_by_id(item_id))
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

        EditRequested { item_id, cursor } => {
            ops::outline_events::edit_requested(model, &item_id, cursor)
        }
        BodyChanged {
            item_id,
            body,
            now_ms,
        } => {
            model.clock_ms = now_ms;
            ops::outline_events::body_changed(model, &item_id, body)
        }
        EditFinished { item_id } => ops::edit::finish_edit(model, &item_id),
        EnterPressed {
            item_id,
            cursor,
            now_ms,
        } => {
            model.clock_ms = now_ms;
            ops::split::handle_enter(model, &item_id, cursor)
        }
        BackspaceAtStart { item_id } => ops::merge::merge_into_previous(model, &item_id),
        DeleteAtEnd { item_id } => ops::merge::merge_next_into(model, &item_id),
        NavigateUp { item_id, cursor } => {
            ops::outline_events::navigate(model, &item_id, cursor, -1)
        }
        NavigateDown { item_id, cursor } => {
            ops::outline_events::navigate(model, &item_id, cursor, 1)
        }
        Indent { item_id } => ops::level::indent(model, &item_id),
        Outdent { item_id } => ops::level::outdent(model, &item_id),
        MoveUp { item_id } => ops::moves::enqueue_move(model, &item_id, MoveDirection::Up),
        MoveDown { item_id } => ops::moves::enqueue_move(model, &item_id, MoveDirection::Down),
        TaskCycle { item_id } => ops::action::cycle_task(model, &item_id),
        CollapseToggle { item_id } => ops::action::toggle_collapsed(model, &item_id),
        ZoomStep { item_id, up } => ops::action::zoom_step(model, &item_id, up),
        ResetZoom { item_id } => ops::action::set_zoom(model, &item_id, 1.0),
        PasteText {
            item_id,
            text,
            html,
            cursor,
        } => ops::paste::paste_text(model, &item_id, text, html, cursor),
        PasteImage {
            item_id,
            file,
            cursor,
        } => ops::paste::paste_image(model, &item_id, file, cursor),
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
            model
                .selection
                .start_from_editor(&mut model.state, &item_id);
            done()
        }
        Undo => ops::undo_replay::perform(model, ops::undo_replay::Direction::Undo),
        Redo => ops::undo_replay::perform(model, ops::undo_replay::Direction::Redo),
        PasteAsTransclusion { item_id } => ops::action::paste_as_transclusion(model, &item_id),
        RemoveTransclusion { item_id } => ops::action::remove_transclusion(model, &item_id),
        SendToStash { item_id } => ops::action::send_to_stash(model, &item_id),
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
        OutlineScrolled {
            scrolled_from_top,
            top_visible_index,
            rendered_start,
            rendered_end,
        } => screens::journal::outline_scrolled(
            model,
            scrolled_from_top,
            top_visible_index,
            rendered_start,
            rendered_end,
        ),
        ScrollToItemRequested { item_id } => {
            Command::notify_shell(crate::effects::UiOp::ScrollToItem {
                item_id,
                smooth: true,
            })
            .into()
        }

        PageTitleSave {
            dir_path,
            file_name,
        } => screens::page::save_title(model, dir_path, file_name),
        DemoteIndex => screens::page::demote_index(model),
        ToggleAtlas => screens::page::toggle_atlas(model),
        AtlasUp => screens::page::atlas_up(model),
        PlaceAtlasCard { page_id, x, y } => screens::atlas::place_card(model, page_id, x, y),
        ScaleAtlasCard { page_id, scale } => screens::atlas::scale_card(model, page_id, scale),
        SetAtlasGroupTitle {
            group_key,
            side,
            title,
        } => screens::atlas::set_group_title(model, group_key, side, title),
        AddAtlasCallout { x, y, side, text } => {
            screens::atlas::add_callout(model, x, y, side, text)
        }
        MoveAtlasCallout { key, x, y } => screens::atlas::move_callout(model, key, x, y),
        ScaleAtlasCallout { key, scale } => screens::atlas::scale_callout(model, key, scale),
        AddAtlasLink { x, y, target } => screens::atlas::add_link(model, x, y, target),
        MoveAtlasLink { item_id, x, y } => screens::atlas::move_link(model, item_id, x, y),
        ScaleAtlasLink { item_id, scale } => screens::atlas::scale_link(model, item_id, scale),
        SetAtlasLink { item_id, target } => screens::atlas::set_link(model, item_id, target),
        RemoveAtlasLink { item_id } => screens::atlas::remove_link(model, item_id),
        SearchAtlasLinks { query } => screens::atlas::search_links(model, query),
        SetAtlasCallout { key, side, text } => screens::atlas::set_callout(model, key, side, text),
        RemoveAtlasCard { page_id } => screens::atlas::remove_card(model, page_id),
        GroupAtlasCards {
            page_ids,
            target_id,
        } => screens::atlas::group_cards(model, page_ids, target_id),
        UngroupAtlasCards { page_ids } => screens::atlas::ungroup_cards(model, page_ids),
        EditPageProperties => screens::page::edit_properties(model),
        SetPageProperty {
            key,
            value_type,
            value,
        } => screens::page::set_property(model, key, value_type, value),
        DeletePageProperty { key } => screens::page::delete_property(model, key),
        CreateNewPage { search_query } => screens::layout::create_new_page(model, search_query),
        DeleteCurrentPage => screens::layout::delete_current_page(model),
        NavigateToToday => Command::notify_shell(crate::effects::NavigateOp {
            url: "/today".into(),
            replace: false,
        })
        .into(),
        NavigateHome => screens::settings::navigate_home(model),
        NavigateToRoot => screens::navigate::to_page(
            &screens::atlas::folder_path(model, "index"),
            false,
            false,
            false,
            None,
        ),
        NavigateToTags => screens::navigate::to_page(
            &screens::atlas::folder_path(model, &format!("{}index", model.config.tags_dir)),
            false,
            false,
            false,
            None,
        ),
        GoToPage {
            path,
            new_tab,
            new_window,
        } => screens::navigate::to_page(
            &screens::atlas::folder_path(model, &path),
            new_tab,
            new_window,
            false,
            None,
        ),
        JournalNav(nav) => screens::journal::nav(model, nav),

        SearchQueryChanged(q) => screens::search::query_changed(model, q),
        SearchKey(key) => screens::search::key(model, key),
        SearchResultClicked { index } => screens::search::result_clicked(model, index),
        SearchDropdown(show) => {
            model.search.show_dropdown = show;
            done()
        }

        IndexSortsRestored(sorts) => {
            model.sidebar.index_sorts = sorts;
            done()
        }
        AtlasDirsRestored(dirs) => {
            model.sidebar.atlas_dirs = dirs.into_iter().collect();
            done()
        }
        SetIndexSort { dir, sort } => {
            model.sidebar.index_sorts.insert(dir, sort);
            done()
        }

        SetWeekNumbering(value) => screens::settings::set_week_numbering(model, value),
        SetLogicalOutdent(value) => screens::settings::set_logical_outdent(model, value),
        SetTagsDir(value) => screens::settings::set_tags_dir(model, value),
        SetHomeLink(value) => screens::settings::set_home_link(model, value),
        BrowseFolder => screens::settings::browse_folder(model),
        AddPropertyDefinition { key, value_type } => {
            screens::settings::add_property_definition(model, key, value_type)
        }
        DeletePropertyDefinition { key } => {
            screens::settings::delete_property_definition(model, key)
        }

        ExtensionCreateItem {
            source,
            request_id,
            page_id,
            position,
            after_item_id,
            level,
            body,
            transclude,
        } => screens::extensions::create_item(
            model,
            source,
            request_id,
            page_id,
            position,
            after_item_id,
            level,
            body,
            transclude,
        ),
        ExtensionConfirm {
            source,
            request_id,
            message,
        } => screens::extensions::confirm(source, request_id, message),
        LaunchUrl { url } => run(move |api| async move {
            if !api.launch_url(&url).await.unwrap_or(false) {
                log_error(format!("Failed to launch app with URL: {url}"));
            }
        }),

        Sse(event) => screens::sse::on_event(model, event),
    }
}
