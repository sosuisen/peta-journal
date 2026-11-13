//! The Crux app: the model, the events the shell sends, and `update`.
//!
//! Operations that wait for the server are written as async commands. A command cannot
//! touch the model, so it sends an [`Event`] that carries the answer as data; `update`
//! hands it to the function that writes it to the model (ADR 011). `update` handles the
//! events a command sends before the next effect reaches the shell.

use crate::effects::{Api, Effect, SseEvent};
use crate::model::*;
use crate::ops;
use crate::outline::{KeyRelay, OutlineState, Selection, UndoStacks};
use crate::rest::ClientId;
use crate::screens;
use crate::util::filename::JournalType;
use crux_core::command::CommandContext;
use crux_core::{render, App, Command};
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::fmt;
use std::future::Future;

pub type Cmd = Command<Effect, Event>;
pub type Ctx = CommandContext<Effect, Event>;

/// What the editor reports about the `[[wiki link]]` being typed: the text inside and where
/// the caret is on the screen.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize, serde::Serialize)]
pub struct WikiLinkContext {
    pub query: String,
    pub x: f64,
    pub y: f64,
}

/// Keys the shell forwards while the outline is in selection mode.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
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

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum SearchKey {
    Down,
    Up,
    Enter,
    Escape,
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum JournalNav {
    Prev,
    Next,
}

/// The URL the shell is showing.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum Route {
    /// `/pages/<name>`, `/docs/<name>`, `/journals/<name>` or `/today` (name `today`).
    Page {
        name: String,
        fragment: Option<String>,
    },
    Settings,
}

impl Route {
    /// The route of a URL. `path` and `fragment` are already decoded, as only the shell knows how
    /// its URLs are encoded. None when the path names nothing.
    pub fn parse(path: &str, fragment: Option<&str>) -> Option<Route> {
        let path = path.trim_start_matches('/');
        if path == "settings" {
            return Some(Route::Settings);
        }
        if path.is_empty() {
            return None;
        }
        let name = ["pages/", "docs/", "journals/"]
            .iter()
            .find_map(|prefix| path.strip_prefix(prefix))
            .map_or(path, |rest| rest.trim_end_matches('/'));
        Some(Route::Page {
            name: if name.is_empty() { "today" } else { name }.to_string(),
            fragment: fragment.filter(|f| !f.is_empty()).map(str::to_string),
        })
    }

    /// The route of a URL that is not encoded, with the item to go to after a `#`. The URLs of
    /// the core's own navigation are like this. A shell with encoded URLs decodes them itself
    /// and calls [`Route::parse`].
    pub fn parse_url(url: &str) -> Option<Route> {
        match url.split_once('#') {
            Some((path, fragment)) => Route::parse(path, Some(fragment)),
            None => Route::parse(url, None),
        }
    }
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Event {
    /// The shell is up: connect, load the settings and the sidebar. `client_id` is the UUID
    /// that the shell made for itself and sends with every change request (ADR-015).
    Start {
        client_id: ClientId,
    },
    /// The URL changed; `today` is the local date as `YYYY-MM-DD`.
    Route {
        route: Route,
        today: String,
    },

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
    /// A key that only hides the children, or only shows them: nothing happens when the
    /// item is already so.
    SetCollapsed {
        item_id: String,
        collapsed: bool,
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
        /// The text goes into the item as it is, e.g. on Ctrl+Shift+V.
        raw: bool,
    },
    /// The server did a paste: what it wrote goes on screen and into the undo history.
    Pasted {
        item_id: String,
        /// Where the cursor was when the text was pasted.
        cursor_before: usize,
        pasted: Pasted,
    },
    /// The page was read again after a paste inside a transclusion: the cursor can go to its
    /// place, or to `fallback` when the page does not show the item it was to go to.
    PastedPageReloaded {
        cursor: PasteCursor,
        fallback: PasteCursor,
    },
    /// The server wrote what an undo or a redo replayed: its answer goes on screen.
    UndoSynced {
        /// The number of the sync this answers; see [`ops::undo_replay::UndoSyncs`].
        sync: u64,
        items: Vec<Item>,
        /// The items whose body or task state the replay changed: they take the server's
        /// parse.
        reparsed: Vec<String>,
        /// The items the batch made again: they are shown as the server made them.
        recreated: Vec<String>,
        /// The empty items the server made on the pages the batch left without an item.
        created: Vec<Item>,
    },
    /// The server split an item for an Enter: what it wrote goes on screen and into the undo
    /// history, and the new item is edited.
    SplitDone {
        item_id: String,
        /// Where the cursor was when Enter was pressed.
        cursor_before: usize,
        split: Box<Split>,
    },
    /// The page was read again after a split inside a transclusion: the edit can move to the
    /// new item, or stay in the item that was split when the page does not show the new one.
    SplitPageReloaded {
        item_id: String,
        created_id: String,
    },
    /// The user said whether an item that other items transclude may be joined away, or
    /// nobody had to be asked: the join goes to the server, or the edit goes on.
    MergeAnswered {
        key: ops::merge::JoinKey,
        /// The item whose editor the key was pressed in.
        item_id: String,
        agreed: bool,
    },
    /// The server joined two items: the joined item goes on screen and is edited, and the
    /// other one goes.
    Merged {
        key: ops::merge::JoinKey,
        /// The item that went away.
        item_id: String,
        merged: Box<Item>,
    },
    /// The server refused a join, or could not be reached: nothing was written.
    MergeFailed {
        /// The items with typed text that no request but the join carried.
        unsaved: Vec<String>,
    },
    /// The server moved items: they go to their new place on screen, and the move goes into
    /// the undo history.
    Moved {
        moved: Vec<Item>,
        /// Where they go among the items of the screen.
        position: ItemPosition,
        /// The item the move was started from, in whose editor the edit goes on. None for
        /// a move of the selection.
        edited: Option<String>,
        /// The label of the undo entry.
        label: ops::moves::MoveLabel,
    },
    /// The server deleted the selected items: the screen shows it now.
    SelectionDeleted {
        item_ids: Vec<String>,
        /// The empty items the server made on the pages the deletion left without an item.
        created: Vec<Item>,
    },
    /// The server moved an item to the journal of another day: the journal follows it or
    /// reads itself again.
    MovedToDay {
        moved: ops::moves::CrossDayMove,
    },
    /// The server moved an item that a transclusion shows: the page is read again.
    MovedInTransclusion {
        item_id: String,
    },
    /// The page was read again after a move inside a transclusion: the edit goes on in the
    /// moved item.
    MovePageReloaded {
        item_id: String,
    },
    /// The server refused the move of an edited item, or could not be reached: nothing was
    /// written.
    MoveFailed {
        /// The item with typed text that no request but the move carried.
        unsaved: Option<String>,
    },
    /// A move is through, with all that follows it on screen: the next one can start.
    MoveDone {
        /// The move kept the blur of the editor from ending the edit until now.
        handover: bool,
    },
    /// The server refused a split, or could not be reached: nothing was written.
    SplitFailed {
        item_id: String,
        /// The item had typed text that no request but the split carried.
        unsaved: bool,
    },
    /// The server refused a paste, or could not be reached: nothing was written.
    PasteFailed {
        item_id: String,
        /// The item had typed text that no request but the paste carried.
        unsaved: bool,
    },
    PasteImage {
        item_id: String,
        file: u64,
        cursor: usize,
    },
    /// The pasted image is on the server: its link is pasted into the item.
    ImageUploaded {
        item_id: String,
        image: Uploaded,
        /// Where the cursor was when the image was pasted.
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
    /// The page names the search the timer `timer_id` started found for a wiki link.
    WikiLinksSuggested {
        timer_id: u64,
        titles: Vec<String>,
    },
    /// The server changed the task state of the item `item_id`, saved under `target_id`.
    TaskCycled {
        item_id: String,
        target_id: String,
        updated: Box<Item>,
    },
    /// The server saved the collapsing or expanding of the item `before`.
    CollapseSaved {
        before: Box<Item>,
    },
    /// The server refused the collapsing or expanding of the item `before`.
    CollapseRefused {
        before: Box<Item>,
    },
    /// The server made an item a transclusion, or a plain item again.
    TransclusionChanged {
        item: Box<Item>,
    },
    /// The pause after typing into the item saved under `target` ended.
    AutoSaveFired {
        target: String,
        timer_id: u64,
    },
    /// The moment given to the auto-save before pages are read again ended.
    PendingReloadsDue,
    /// A body is on its way to the server, with an open code fence closed.
    BodyShown {
        item_id: String,
        body: String,
    },
    /// The server saved a body and answered with the item.
    BodySaved {
        item: Box<Item>,
    },
    /// The server saved the source of the transclusion root `root_id`.
    RootParsed {
        root_id: String,
        source: Box<Item>,
    },
    /// A handler that kept the blur of an editor from ending an edit is done.
    HandoverEnded,
    /// The item left with an arrow key is saved: the edit moves on to the next item.
    EditorSwitched {
        from_id: String,
        to_id: String,
        cursor: usize,
    },
    /// The save that ends the edit of an item is over: the item goes to view mode.
    EditorClosed {
        item_id: String,
    },
    /// The server answered a page that is read again.
    PageReplaced {
        page_id: String,
        loaded: Box<PageWithItems>,
    },
    /// The shown page is to be read from the server again, e.g. after an undo did not
    /// reach the server.
    ReloadShownPage,
    /// Items were copied to the system clipboard; the app keeps both forms of the text.
    MarkdownCopied {
        flat_markdown: String,
        tree_markdown: String,
    },
    /// The server refused an indent or outdent of items of these pages.
    LevelShiftRefused {
        page_ids: HashSet<String>,
    },
    WikiPick {
        title: String,
    },
    /// Opens the wiki link or the tag at the cursor of an item.
    OpenLinkAt {
        item_id: String,
        cursor: usize,
    },
    ShiftSelect {
        item_id: String,
    },
    /// Deletes one item with everything under it, without selecting it first.
    DeleteItem {
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
    /// The server sent an item to the stash: the heading and the items go to their place.
    Stashed {
        stashed: Box<Stashed>,
    },
    /// The stash did not reach the server: the typed text it was to carry is saved again.
    StashFailed {
        unsaved: Option<String>,
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
    /// The server answered the journal of `date`; `title` is the title of the window for it.
    JournalLoaded {
        date: String,
        title: String,
        loaded: Box<PageWithItems>,
    },
    /// The server answered the journal before one that is shown; the load goes on until
    /// the outline holds `target_item_count` items.
    OlderJournalLoaded {
        weekly: bool,
        target_item_count: usize,
        name: String,
        items: Vec<Item>,
        transcluded_trees: Option<TranscludedTrees>,
    },
    /// The load of the older journals that fill the outline ended.
    PreviousJournalsLoaded,
    /// The wait for the day an item moved to ended: the item is edited there.
    MovedItemShown {
        item_id: String,
    },
    /// The server knows the opened page under another path, e.g. a folder as its index page.
    PageRedirected {
        path: String,
    },
    /// The server answered the doc page of the path the screen opened; `created` when it
    /// made the page.
    PageLoaded {
        page: Page,
        created: bool,
    },
    /// The server answered whether the shown index page has no pages under it.
    CanDemoteLoaded {
        can_demote: bool,
    },
    /// The server answered the items of a doc page.
    PageItemsLoaded {
        page_id: String,
        loaded: Box<PageWithItems>,
    },
    /// The server answered the backlinks of a doc page.
    BacklinksLoaded {
        page_id: String,
        entries: Vec<BacklinkEntry>,
    },
    /// The server saved the new title or folder of the shown page.
    TitleSaved {
        page: Page,
    },
    /// The server saved a property change of a doc page and answered with the page.
    PagePropertySaved {
        page: Page,
    },
    /// The server refused a property change of a doc page; `before` is what the page had.
    PagePropertyRefused {
        page_id: String,
        before: std::collections::BTreeMap<String, crate::model::PropertyValue>,
    },
    /// The server demoted the shown index page: the page opens under its new path.
    Demoted {
        path: String,
    },
    /// The map button: switches between the index page and the atlas page of a directory.
    ToggleAtlas,
    /// The up button of an atlas page: opens the atlas of the directory above.
    AtlasUp,
    /// A card dropped on the atlas canvas: the card is placed with its center there. A page
    /// card is named by its page, a link card by its item.
    PlaceAtlasCard {
        card_id: String,
        x: f64,
        y: f64,
    },
    /// The selected cards scaled with Ctrl and the wheel: the card takes the scale.
    ScaleAtlasCard {
        card_id: String,
        scale: f64,
    },
    /// The group menu's "Edit Title": the group's title lies along a side of its outline.
    /// An empty title takes it away.
    SetAtlasGroupTitle {
        group_id: String,
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
        card_ids: Vec<String>,
        target_id: String,
    },
    /// Cards dragged out of the outline of their group with Ctrl held: they leave it.
    UngroupAtlasCards {
        card_ids: Vec<String>,
    },
    /// The server refused the new bounds of a card or a callout; `before` is what it had.
    AtlasBoundsRefused {
        item_id: String,
        before: Option<crate::model::PropertyValue>,
    },
    /// The server changed the outline of an atlas page and answered with its items.
    AtlasChanged {
        atlas_id: String,
        items: Vec<Item>,
    },
    /// The page names the search the timer `timer_id` started found for the dialog of a link.
    AtlasLinksSuggested {
        timer_id: u64,
        titles: Vec<String>,
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
    /// The server deleted the shown page.
    PageDeleted,
    NavigateToToday,
    /// A page is to be opened by its path, e.g. one just named in the header or the search.
    OpenPage {
        path: String,
    },
    /// The server answered the search the debounce timer `timer_id` started.
    SearchAnswered {
        timer_id: u64,
        query: String,
        results: Vec<SearchResult>,
    },
    /// The search the debounce timer `timer_id` started failed.
    SearchFailed {
        timer_id: u64,
    },
    /// The server answered the ten most recent doc pages.
    RecentPagesLoaded {
        pages: Vec<Page>,
    },
    /// The server answered the pages of the directory the index list shows.
    IndexPagesLoaded {
        header: String,
        is_root: bool,
        pages: Vec<Page>,
    },
    /// The wait before the next try to connect the event stream ended.
    SseReconnect,
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
        position: usize,
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
    /// The server answered with its settings.
    ConfigLoaded {
        config: ServerConfig,
    },
    /// The server answered with the property definitions: when they are loaded, and after a
    /// change of them.
    PropertyDefinitionsLoaded {
        definitions: Vec<PropertyDefinition>,
    },
    /// The server refused a change of the property definitions.
    PropertyDefinitionRefused {
        reason: String,
    },
    /// The server saved the tag directory and answered with the form it keeps.
    TagsDirSaved {
        tags_dir: String,
    },
    /// The server saved the page of the Home button and answered with the form it keeps.
    HomeLinkSaved {
        home_link: String,
    },
    /// A folder was chosen for the markdown files.
    MarkdownDirChosen {
        path: String,
    },

    // ---- links ----
    LaunchUrl {
        url: String,
    },
    /// Shows the folder of the page's markdown file in the file manager of the OS.
    OpenExportFolder {
        page_id: String,
    },

    // ---- from the shell's streams ----
    Sse(SseEvent),
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

/// Stops a debounce timer that is still running and forgets it.
pub fn cancel_timer(timer_id: &mut u64) -> Cmd {
    if *timer_id == 0 {
        return done();
    }
    let id = std::mem::take(timer_id);
    Command::notify_shell(crate::effects::TimerOp::Cancel { id }).into()
}

pub fn log_error(message: impl fmt::Display) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = message;
    }
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("[client-core] {message}");
}

// ---- Model ----

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
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
    pub searching: bool,
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
    /// The name the core last gave to what is open; see `screens::navigate::set_title`.
    pub title: String,
    pub recent_pages: Vec<Page>,
    pub sidebar: SidebarState,
    pub search: SearchState,
    pub wiki: WikiState,
    pub atlas_link_search: AtlasLinkSearch,
    pub config: ConfigState,
    pub connection: ConnectionState,
    /// The ID this client names itself with. The server sends it back with the changes this
    /// client made, so they are told apart from those of the other clients. Until the shell
    /// starts the core, this is an ID that no client has sent.
    pub client_id: ClientId,
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
    pub undo_syncs: ops::undo_replay::UndoSyncs,
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
            title: String::new(),
            recent_pages: Vec::new(),
            sidebar: SidebarState::default(),
            search: SearchState::default(),
            wiki: WikiState::default(),
            atlas_link_search: AtlasLinkSearch::default(),
            config: ConfigState::default(),
            connection: ConnectionState::Disconnected,
            client_id: ClientId::new(),
            sse_reconnect_attempts: 0,
            scrolled_from_top: false,
            clipboard: None,
            today: String::new(),
            hmtid: hmtid::Generator::new('-', true),
            clock_ms: 0,
            next_timer_id: 1,
            undo_syncs: Default::default(),
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

    /// A copy of the item as the state shows it, for a command that goes on after the
    /// state has changed.
    pub fn cloned_item(&self, item_id: &str) -> Option<Item> {
        self.current_item(item_id).map(|item| (*item).clone())
    }

    /// A fresh HMTID for an item created on the client.
    pub fn new_item_id(&self, now_ms: u64) -> String {
        self.hmtid.generate_at(now_ms).as_str().to_string()
    }
}

// ---- The app ----

/// The app, whose rows give their bodies to the shell as `B` (ADR 042): the AST for the web
/// shell, and `RichText` for a native shell.
pub struct PetaApp<B = flatmarkdown_ast::Node> {
    /// What the outline showed last of each body.
    bodies: crate::body_view::Bodies<B>,
}

impl<B> Default for PetaApp<B> {
    fn default() -> Self {
        PetaApp {
            bodies: Default::default(),
        }
    }
}

impl<B: crate::body_view::BodyView> App for PetaApp<B> {
    type Event = Event;
    type Model = Model;
    type ViewModel = crate::view::ViewModel<B>;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Cmd {
        let cmd = dispatch(event, model);
        let flush = ops::file_reload::schedule_pending_flush(model);
        Command::all([cmd, flush, render::render()])
    }

    fn view(&self, model: &Model) -> Self::ViewModel {
        crate::view::build(model, &self.bodies)
    }
}

fn dispatch(event: Event, model: &mut Model) -> Cmd {
    use Event::*;
    match event {
        Start { client_id } => {
            model.client_id = client_id;
            screens::start::start(model)
        }
        Route { route, today } => screens::route::route(model, route, today),

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
        SplitDone {
            item_id,
            cursor_before,
            split,
        } => ops::split::show_split(model, &item_id, cursor_before, *split),
        SplitPageReloaded {
            item_id,
            created_id,
        } => ops::split::focus_after_split(model, &item_id, &created_id),
        SplitFailed { item_id, unsaved } => ops::split::split_failed(model, &item_id, unsaved),
        BackspaceAtStart { item_id } => ops::merge::merge_into_previous(model, &item_id),
        DeleteAtEnd { item_id } => ops::merge::merge_next_into(model, &item_id),
        MergeAnswered {
            key,
            item_id,
            agreed,
        } => ops::merge::merge_answered(model, key, &item_id, agreed),
        Merged {
            key,
            item_id,
            merged,
        } => ops::merge::show_merged(model, key, &item_id, *merged),
        MergeFailed { unsaved } => ops::merge::merge_failed(model, &unsaved),
        NavigateUp { item_id, cursor } => {
            ops::outline_events::navigate(model, &item_id, cursor, -1)
        }
        NavigateDown { item_id, cursor } => {
            ops::outline_events::navigate(model, &item_id, cursor, 1)
        }
        Indent { item_id } => ops::level::indent(model, &item_id),
        Outdent { item_id } => ops::level::outdent(model, &item_id),
        Moved {
            moved,
            position,
            edited,
            label,
        } => ops::moves::show_moved(model, moved, &position, edited, label),
        SelectionDeleted { item_ids, created } => {
            ops::action::show_deleted(model, &item_ids, created)
        }
        MovedToDay { moved } => ops::moves::show_moved_to_day(model, moved),
        MovedInTransclusion { item_id } => ops::moves::show_moved_in_transclusion(model, item_id),
        MovePageReloaded { item_id } => ops::moves::focus_moved(model, item_id),
        MoveFailed { unsaved } => ops::moves::move_failed(model, unsaved.as_deref()),
        MoveDone { handover } => ops::moves::move_done(model, handover),
        MoveUp { item_id } => ops::moves::enqueue_move(model, &item_id, MoveDirection::Up),
        MoveDown { item_id } => ops::moves::enqueue_move(model, &item_id, MoveDirection::Down),
        TaskCycle { item_id } => ops::action::cycle_task(model, &item_id),
        CollapseToggle { item_id } => ops::action::toggle_collapsed(model, &item_id),
        SetCollapsed { item_id, collapsed } => {
            ops::action::set_collapsed(model, &item_id, collapsed)
        }
        ZoomStep { item_id, up } => ops::action::zoom_step(model, &item_id, up),
        ResetZoom { item_id } => ops::action::set_zoom(model, &item_id, 1.0),
        PasteText {
            item_id,
            text,
            html,
            cursor,
            raw,
        } => ops::paste::paste_text(model, &item_id, text, html, cursor, raw),
        Pasted {
            item_id,
            cursor_before,
            pasted,
        } => ops::paste::show_pasted(model, &item_id, cursor_before, pasted),
        PastedPageReloaded { cursor, fallback } => {
            ops::paste::place_cursor(model, cursor, fallback)
        }
        PasteFailed { item_id, unsaved } => ops::paste::paste_failed(model, &item_id, unsaved),
        UndoSynced {
            sync,
            items,
            reparsed,
            recreated,
            created,
        } => ops::undo_replay::show_synced(model, sync, items, reparsed, recreated, created),
        PasteImage {
            item_id,
            file,
            cursor,
        } => ops::paste::paste_image(&item_id, file, cursor),
        ImageUploaded {
            item_id,
            image,
            cursor,
        } => ops::paste::link_image(model, &item_id, &image, cursor),
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
        WikiLinksSuggested { timer_id, titles } => {
            ops::wiki::show_suggestions(model, timer_id, titles)
        }
        TaskCycled {
            item_id,
            target_id,
            updated,
        } => ops::action::show_task_cycled(model, &item_id, &target_id, *updated),
        CollapseSaved { before } => ops::action::record_collapse(model, *before),
        CollapseRefused { before } => ops::action::put_collapse_back(model, *before),
        TransclusionChanged { item } => ops::action::show_transclusion_change(model, &item),
        ReloadShownPage => ops::file_reload::reload_shown_page(model),
        MarkdownCopied {
            flat_markdown,
            tree_markdown,
        } => {
            model.clipboard = Some((flat_markdown, tree_markdown));
            done()
        }
        LevelShiftRefused { page_ids } => ops::level::reload_after_refused_shift(model, page_ids),
        AutoSaveFired { target, timer_id } => ops::edit::auto_save(model, target, timer_id),
        PendingReloadsDue => ops::file_reload::flush_pending(model),
        BodyShown { item_id, body } => ops::edit::show_body(model, &item_id, body),
        BodySaved { item } => ops::edit::show_saved_body(model, &item),
        RootParsed { root_id, source } => ops::edit::show_root_parsed(model, &root_id, *source),
        HandoverEnded => ops::edit::end_handover(model),
        EditorSwitched {
            from_id,
            to_id,
            cursor,
        } => ops::outline_events::switch_editor(model, &from_id, &to_id, cursor),
        EditorClosed { item_id } => {
            model.state.remove_editing_item(&item_id);
            done()
        }
        PageReplaced { page_id, loaded } => {
            ops::file_reload::show_replaced_page(model, &page_id, *loaded)
        }
        WikiPick { title } => ops::wiki::pick(model, &title),
        OpenLinkAt { item_id, cursor } => ops::wiki::open_link_at(model, &item_id, cursor),
        ShiftSelect { item_id } => {
            model
                .selection
                .start_from_editor(&mut model.state, &item_id);
            done()
        }
        DeleteItem { item_id } => ops::outline_events::delete_item(model, &item_id),
        Undo => ops::undo_replay::perform(model, ops::undo_replay::Direction::Undo),
        Redo => ops::undo_replay::perform(model, ops::undo_replay::Direction::Redo),
        PasteAsTransclusion { item_id } => ops::action::paste_as_transclusion(model, &item_id),
        RemoveTransclusion { item_id } => ops::action::remove_transclusion(model, &item_id),
        SendToStash { item_id } => ops::action::send_to_stash(model, &item_id),
        Stashed { stashed } => ops::action::show_stashed(model, *stashed),
        StashFailed { unsaved } => ops::action::stash_failed(model, unsaved.as_deref()),
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
        Demoted { path } => screens::page::show_demoted(&path),
        JournalLoaded {
            date,
            title,
            loaded,
        } => screens::journal::journal_loaded(model, date, title, *loaded),
        OlderJournalLoaded {
            weekly,
            target_item_count,
            name,
            items,
            transcluded_trees,
        } => screens::journal::show_older_journal(
            model,
            weekly,
            target_item_count,
            screens::journal::OlderJournal {
                name,
                items,
                transcluded_trees,
            },
        ),
        PreviousJournalsLoaded => screens::journal::previous_journals_loaded(model),
        MovedItemShown { item_id } => screens::journal::edit_moved_item(model, &item_id),
        PageRedirected { path } => screens::navigate::replace_page(&path),
        PageLoaded { page, created } => screens::page::show_page(model, page, created),
        CanDemoteLoaded { can_demote } => screens::page::show_can_demote(model, can_demote),
        PageItemsLoaded { page_id, loaded } => {
            screens::page::show_page_items(model, &page_id, *loaded)
        }
        BacklinksLoaded { page_id, entries } => {
            screens::page::show_backlinks(model, &page_id, entries)
        }
        TitleSaved { page } => screens::page::show_title_saved(model, page),
        PagePropertySaved { page } => screens::page::show_property_saved(model, page),
        PagePropertyRefused { page_id, before } => {
            screens::page::put_properties_back(model, &page_id, before)
        }
        ToggleAtlas => screens::page::toggle_atlas(model),
        AtlasUp => screens::page::atlas_up(model),
        PlaceAtlasCard { card_id, x, y } => screens::atlas::place_card(model, card_id, x, y),
        ScaleAtlasCard { card_id, scale } => screens::atlas::scale_card(model, card_id, scale),
        SetAtlasGroupTitle {
            group_id,
            side,
            title,
        } => screens::atlas::set_group_title(model, group_id, side, title),
        AddAtlasCallout { x, y, side, text } => {
            screens::atlas::add_callout(model, x, y, side, text)
        }
        MoveAtlasCallout { key, x, y } => screens::atlas::move_callout(model, key, x, y),
        ScaleAtlasCallout { key, scale } => screens::atlas::scale_callout(model, key, scale),
        AddAtlasLink { x, y, target } => screens::atlas::add_link(model, x, y, target),
        SetAtlasLink { item_id, target } => screens::atlas::set_link(model, item_id, target),
        RemoveAtlasLink { item_id } => screens::atlas::remove_link(model, item_id),
        SearchAtlasLinks { query } => screens::atlas::search_links(model, query),
        SetAtlasCallout { key, side, text } => screens::atlas::set_callout(model, key, side, text),
        RemoveAtlasCard { page_id } => screens::atlas::remove_card(model, page_id),
        GroupAtlasCards {
            card_ids,
            target_id,
        } => screens::atlas::group_cards(model, card_ids, target_id),
        UngroupAtlasCards { card_ids } => screens::atlas::ungroup_cards(model, card_ids),
        AtlasBoundsRefused { item_id, before } => {
            screens::atlas::put_bounds_back(model, &item_id, before)
        }
        AtlasChanged { atlas_id, items } => screens::atlas::show_changed(model, &atlas_id, items),
        AtlasLinksSuggested { timer_id, titles } => {
            screens::atlas::show_link_suggestions(model, timer_id, titles)
        }
        EditPageProperties => screens::page::edit_properties(model),
        SetPageProperty {
            key,
            value_type,
            value,
        } => screens::page::set_property(model, key, value_type, value),
        DeletePageProperty { key } => screens::page::delete_property(model, key),
        CreateNewPage { search_query } => screens::layout::create_new_page(model, search_query),
        DeleteCurrentPage => screens::layout::delete_current_page(model),
        PageDeleted => screens::layout::show_page_deleted(),
        NavigateToToday => screens::navigate::to_url("/today", false),
        OpenPage { path } => screens::navigate::open_page(&path),
        SearchAnswered {
            timer_id,
            query,
            results,
        } => screens::search::show_results(model, timer_id, &query, results),
        SearchFailed { timer_id } => screens::search::search_failed(model, timer_id),
        RecentPagesLoaded { pages } => {
            model.recent_pages = pages;
            done()
        }
        IndexPagesLoaded {
            header,
            is_root,
            pages,
        } => screens::sidebar::show_index_pages(model, header, is_root, pages),
        SseReconnect => screens::sse::connect(model),
        NavigateHome => screens::settings::navigate_home(model),
        NavigateToRoot => screens::navigate::open_folder(model, "index"),
        NavigateToTags => {
            screens::navigate::open_folder(model, &format!("{}index", model.config.tags_dir))
        }
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
        SearchResultClicked { position } => screens::search::result_clicked(model, position),
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
        ConfigLoaded { config } => screens::settings::show_config(model, config),
        PropertyDefinitionsLoaded { definitions } => {
            model.config.property_definitions = definitions;
            done()
        }
        PropertyDefinitionRefused { reason } => {
            model.config.definition_error = Some(reason);
            done()
        }
        TagsDirSaved { tags_dir } => {
            model.config.tags_dir = tags_dir;
            done()
        }
        HomeLinkSaved { home_link } => {
            model.config.home_link = home_link;
            done()
        }
        MarkdownDirChosen { path } => {
            model.config.markdown_dir = path;
            done()
        }

        LaunchUrl { url } => run(move |api| async move {
            if !api.launch_url(&url).await.unwrap_or(false) {
                log_error(format!("Failed to launch app with URL: {url}"));
            }
        }),
        OpenExportFolder { page_id } => run(move |api| async move {
            if let Err(e) = api.open_export_folder(&page_id).await {
                log_error(format!(
                    "Failed to open the export folder of {page_id}: {e}"
                ));
            }
        }),

        Sse(event) => screens::sse::on_event(model, event),
    }
}
