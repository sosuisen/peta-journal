//! The JSON lines this process and `peta-journal.el` send each other.

use crate::emacs_text::EmacsText;
use client_core::app::SelectionKey as CoreSelectionKey;
use client_core::rich_text::RichLink;
use client_core::view::{
    BacklinkGroup, HeadingsView, IndexPagesView, PageProperty, RecentPagesView, Row,
    SearchEntry as CoreSearchEntry, SearchKind as CoreSearchKind, SidebarPage, TaskState,
};
use facet::Facet;

/// Reads one line from Emacs. The error says what is wrong with the line.
pub fn read_line(line: &str) -> Result<Envelope, String> {
    // The line is read twice, as facet-json 0.46 does not rename the variants of an enum
    // under `flatten`. Each read leaves out the fields of the other
    let Sequence { seq } = facet_json::from_str(line).map_err(|e| e.to_string())?;
    let message = facet_json::from_str(line).map_err(|e| e.to_string())?;
    Ok(Envelope { seq, message })
}

/// The number of a line from Emacs, read apart from what the line asks for.
#[derive(Facet)]
struct Sequence {
    seq: u64,
}

/// Writes a message as one line for Emacs, without the line break.
pub fn write_line(message: &Outbound) -> String {
    facet_json::to_string(message).expect("a message is always valid JSON")
}

/// One line from Emacs. Emacs numbers its lines, so it can tell which of them an answer has seen.
#[derive(Debug, PartialEq)]
pub struct Envelope {
    pub seq: u64,
    pub message: Inbound,
}

/// What the user did in Emacs.
#[derive(Debug, PartialEq, Facet)]
#[facet(tag = "type", rename_all = "snake_case")]
#[repr(C)]
pub enum Inbound {
    Start {
        url: String,
    },
    /// Starts at the page `path`, a name as the user typed it. Empty for the journal of today.
    StartAtPage {
        path: String,
    },
    Go {
        url: String,
    },
    Back,
    /// Goes forward again to the page left by going back.
    Forward,
    GoToPage {
        path: String,
    },
    /// Opens the journal of the day `date`, or the page of its week, month or year. Emacs
    /// picks the date in its calendar.
    GoToDate {
        date: String,
        unit: DateUnit,
    },
    /// Gives the page on screen this path, with its folder.
    RenamePage {
        path: String,
    },
    /// Gives the index page on screen this folder, with the folders above it.
    RenameFolder {
        path: String,
    },
    /// Makes the index page on screen the page of the name of its folder.
    DemoteIndex,
    /// Deletes the page on screen. The core asks first.
    DeletePage,
    /// The day may have changed while the core could not look at the clock, as when Emacs
    /// comes back to the front after the PC slept (#1031).
    DayMayHaveChanged,
    SetProperty {
        key: String,
        value: String,
    },
    DeleteProperty {
        key: String,
    },
    OpenLinkAt {
        id: String,
        cursor: usize,
    },
    /// The user clicked a link of a shown row. The core decides how it opens (ADR 083).
    LinkClicked {
        link: RichLink,
    },
    JournalNav {
        forward: bool,
    },
    EditRequested {
        id: String,
        cursor: usize,
    },
    EditFinished {
        id: String,
    },
    /// Starts the edit of the item `id` with the cursor after `letter` letters of its shown
    /// text, where the point came into its row (ADR 077). The core tells the place in the
    /// Markdown as [`Outbound::Focus`].
    EditAtLetter {
        id: String,
        letter: usize,
    },
    BodyChanged {
        id: String,
        body: String,
    },
    /// A `[`, a `]` or a backtick was typed. It may close a link or a code fence.
    Typed {
        id: String,
        cursor: usize,
    },
    Enter {
        id: String,
        cursor: usize,
    },
    BackspaceAtStart {
        id: String,
    },
    DeleteAtEnd {
        id: String,
    },
    Indent {
        id: String,
    },
    Outdent {
        id: String,
    },
    MoveUp {
        id: String,
    },
    MoveDown {
        id: String,
    },
    TaskCycle {
        id: String,
    },
    CollapseToggle {
        id: String,
    },
    Paste {
        id: String,
        text: String,
        cursor: usize,
    },
    InsertImage {
        id: String,
        path: String,
        cursor: usize,
    },
    DeleteItem {
        id: String,
    },
    /// Selects the item with its children. The edit ends, and the keys of the selection
    /// come as `SelectionKey` until it ends.
    Select {
        id: String,
    },
    SelectionKey {
        key: SelectionKey,
    },
    /// Makes the item a transclusion of the item whose id is `text`, which Emacs took from
    /// its clipboard. Text that is not an item id does nothing.
    PasteAsTransclusion {
        id: String,
        text: String,
    },
    /// Makes a transclusion a plain item again.
    RemoveTransclusion {
        id: String,
    },
    /// Sends the item with its children under the `# Stash` heading at the end of its page.
    SendToStash {
        id: String,
    },
    Undo,
    Redo,
    Scrolled {
        top: usize,
        bottom: usize,
    },
    Search {
        query: String,
    },
    SearchResult {
        position: usize,
    },
    Answer {
        req: u64,
        yes: bool,
    },
    /// Asks for the pages whose names match `query`, the text typed in a wiki link of the
    /// item. The answer is [`Outbound::PageNames`].
    PageNames {
        id: String,
        query: String,
    },
}

/// The keys of the selection mode, by the names of the core's keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Facet)]
#[repr(C)]
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

impl From<SelectionKey> for CoreSelectionKey {
    fn from(key: SelectionKey) -> CoreSelectionKey {
        match key {
            SelectionKey::Escape => CoreSelectionKey::Escape,
            SelectionKey::Delete => CoreSelectionKey::Delete,
            SelectionKey::MoveUp => CoreSelectionKey::MoveUp,
            SelectionKey::MoveDown => CoreSelectionKey::MoveDown,
            SelectionKey::Indent => CoreSelectionKey::Indent,
            SelectionKey::Outdent => CoreSelectionKey::Outdent,
            SelectionKey::ExtendUp => CoreSelectionKey::ExtendUp,
            SelectionKey::ExtendDown => CoreSelectionKey::ExtendDown,
            SelectionKey::Copy => CoreSelectionKey::Copy,
            SelectionKey::Cut => CoreSelectionKey::Cut,
            SelectionKey::Undo => CoreSelectionKey::Undo,
            SelectionKey::Redo => CoreSelectionKey::Redo,
        }
    }
}

/// Which page of a date to open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Facet)]
#[facet(rename_all = "snake_case")]
#[repr(C)]
pub enum DateUnit {
    Day,
    Week,
    Month,
    Year,
}

/// What Emacs is told to do.
#[derive(Debug, Clone, PartialEq, Facet)]
#[facet(tag = "t", rename_all = "snake_case")]
#[repr(C)]
pub enum Outbound {
    /// Which line of Emacs was handled last, and whether the core holds keys for an editor that
    /// is not there yet. Emacs waits on this after a key that changes the outline.
    State {
        ack: u64,
        holding: bool,
    },
    /// What changed in the rows since Emacs was last told, to apply in order (ADR 078).
    Rows {
        ack: u64,
        page_id: Option<String>,
        changes: Vec<RowChange>,
    },
    /// Puts the point in an item. Without a cursor, a point already in the item stays.
    Focus {
        ack: u64,
        id: String,
        cursor: Option<usize>,
    },
    Scroll {
        id: String,
    },
    Title {
        title: String,
    },
    /// The path of the page on screen, when the page can be renamed.
    PagePath {
        path: Option<String>,
    },
    /// The folder of the index page on screen, as `a/b`. `None` when the screen is not an index
    /// page whose folder can change, such as the index of the root. `demote_name` is the name
    /// the page takes when it is demoted, when no page is under it.
    Index {
        folder: Option<String>,
        demote_name: Option<String>,
    },
    /// The properties of the page on screen. `rows` is `None` on a screen that cannot have
    /// properties, such as a journal. `free_keys` can be set on the page and are not on it yet.
    /// `datetime_keys` are the keys of both whose value is a date and a time.
    Properties {
        rows: Option<Vec<PropertyRow>>,
        free_keys: Vec<FreeKey>,
        datetime_keys: Vec<String>,
    },
    Status {
        connection: String,
    },
    Search {
        results: Vec<SearchEntry>,
        /// The results are still those of an older query, as the answer to this one is on its way.
        searching: bool,
    },
    Recent {
        paths: Vec<String>,
    },
    /// What the sidebar lists, in the order of the web frontend.
    Sidebar(SidebarView),
    Backlinks {
        groups: Vec<BacklinkEntry>,
    },
    Message {
        text: String,
    },
    /// Text for the kill ring, such as the Markdown of the copied items.
    Copy {
        text: String,
    },
    /// An address that the core ordered to open in the browser (ADR 083). An address that
    /// starts with `/` is on the journal server, which only Emacs knows.
    Browse {
        address: String,
    },
    /// The pages whose names match `query`, for the completion of a wiki link. It is sent
    /// even when nothing was found, as Emacs waits for it.
    PageNames {
        query: String,
        titles: Vec<String>,
    },
    /// A yes-or-no question for the user. Emacs answers with [`Inbound::Answer`].
    Ask {
        req: u64,
        message: String,
    },
}

/// One line of the search results. `position` is what [`Inbound::SearchResult`] takes.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct SearchEntry {
    pub position: usize,
    pub kind: SearchKind,
    /// The path of the page, or of the page to create.
    pub title: String,
    pub label: String,
    pub snippet: String,
}

impl SearchEntry {
    /// The line of `entry`, which is at `position` in the results.
    pub fn of(position: usize, entry: &CoreSearchEntry) -> SearchEntry {
        SearchEntry {
            position,
            kind: SearchKind::of(entry.kind),
            title: entry.title.clone(),
            label: entry.label.clone(),
            snippet: entry.snippet.clone(),
        }
    }
}

/// What a line of the search results opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Facet)]
#[facet(rename_all = "snake_case")]
#[repr(C)]
pub enum SearchKind {
    /// Today's journal.
    Today,
    Page,
    Create,
    Item,
}

impl SearchKind {
    pub fn of(kind: CoreSearchKind) -> SearchKind {
        match kind {
            CoreSearchKind::Today => SearchKind::Today,
            CoreSearchKind::Page => SearchKind::Page,
            CoreSearchKind::Create => SearchKind::Create,
            CoreSearchKind::Item => SearchKind::Item,
        }
    }
}

client_core::list_change!(
    /// One change to the rows of the outline, as Emacs applies it to the buffer.
    #[derive(Debug, Clone, PartialEq, Facet)]
    #[repr(C)]
    RowChange,
    RowView
);

/// One row of the outline, as Emacs draws it.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct RowView {
    pub id: String,
    pub level: i32,
    /// The Markdown of the body, which Emacs shows while the item is edited (ADR 077).
    pub body: String,
    /// The body as Emacs shows it while the item is not edited.
    pub text: EmacsText,
    pub bullet: Bullet,
    /// The state of the task, which Emacs marks behind the bullet. None when the item is
    /// not a task.
    pub task: Option<TaskMark>,
    pub collapsed: bool,
    /// The date above the first item of a journal day.
    pub header: Option<String>,
    pub selected: bool,
}

impl RowView {
    /// The row `row`, with `text` as its shown body.
    pub fn of<B>(row: &Row<B>, text: EmacsText) -> RowView {
        RowView {
            id: row.item.id.clone(),
            level: row.item.level,
            body: row.item.body.clone(),
            text,
            bullet: Bullet::of(
                row.is_broken_transclusion,
                row.is_transclusion,
                row.show_bullet,
            ),
            task: TaskMark::of(row.task),
            collapsed: row.is_collapsed && row.has_children,
            header: row.date_header.clone(),
            selected: row.is_selected,
        }
    }
}

/// What the mark at the start of a row shows. A task has the bullet of its kind too: its
/// state is a mark of its own, as in the web frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Facet)]
#[facet(rename_all = "snake_case")]
#[repr(C)]
pub enum Bullet {
    Plain,
    Transclusion,
    Broken,
    /// No bullet: the core shows none before an empty item that is not edited (#1042).
    Hidden,
}

impl Bullet {
    pub fn of(broken: bool, transclusion: bool, shown: bool) -> Bullet {
        if !shown {
            Bullet::Hidden
        } else if broken {
            Bullet::Broken
        } else if transclusion {
            Bullet::Transclusion
        } else {
            Bullet::Plain
        }
    }
}

/// The state of a task, for the mark behind the bullet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Facet)]
#[facet(rename_all = "snake_case")]
#[repr(C)]
pub enum TaskMark {
    Task,
    Running,
    Done,
}

impl TaskMark {
    pub fn of(task: Option<TaskState>) -> Option<TaskMark> {
        task.map(|state| match state {
            TaskState::NotStarted => TaskMark::Task,
            TaskState::Running => TaskMark::Running,
            TaskState::Done => TaskMark::Done,
        })
    }
}

/// One property of the shown page.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct PropertyRow {
    pub key: String,
    /// What the list shows for the key: the body of its name and its type, as
    /// `rating (number)` (ADR 047).
    pub label: String,
    pub value: String,
    pub removable: bool,
    pub editable: bool,
}

impl PropertyRow {
    pub fn of(property: &PageProperty, label: String) -> PropertyRow {
        PropertyRow {
            key: property.key.clone(),
            label,
            value: property.value.clone(),
            removable: property.removable,
            editable: property.editable,
        }
    }
}

/// A key that the shown page can still take, with what Emacs shows for it (see
/// [`PropertyRow::label`]).
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct FreeKey {
    pub key: String,
    pub label: String,
}

/// The sections of the sidebar. The calendar of the web frontend is left out: Emacs has its
/// own (`peta-journal-calendar`).
#[derive(Debug, Clone, PartialEq, Default, Facet)]
pub struct SidebarView {
    /// The pages of the folder of the shown page. `None` when there is nothing to list.
    pub index: Option<SidebarIndex>,
    pub headings: Vec<SidebarHeading>,
    pub recent: Vec<SidebarLine>,
}

#[derive(Debug, Clone, PartialEq, Facet)]
pub struct SidebarIndex {
    /// The folder, as `a/b/`, or `(root)/`.
    pub header: String,
    /// The path of the index page that the header opens.
    pub path: String,
    /// The path that the `../` line opens. `None` in the root.
    pub parent: Option<String>,
    pub pages: Vec<SidebarLine>,
}

/// A page of a list of the sidebar, which opens by its path.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct SidebarLine {
    pub label: String,
    /// The folder shown before the label, for a recent page in a folder.
    pub dir: Option<String>,
    pub path: String,
    pub folder: bool,
    /// The page is the one on screen.
    pub current: bool,
}

#[derive(Debug, Clone, PartialEq, Facet)]
pub struct SidebarHeading {
    pub id: String,
    pub text: String,
    pub level: u8,
}

impl SidebarView {
    pub fn of(
        index: &IndexPagesView,
        headings: &HeadingsView,
        recent: &RecentPagesView,
    ) -> SidebarView {
        // As in the web frontend, only the index list marks the page on screen
        let current = index.current_page_id.as_deref();
        let lines = |pages: &[SidebarPage], marked: bool| -> Vec<SidebarLine> {
            pages
                .iter()
                .map(|page| SidebarLine {
                    label: page.label.clone(),
                    dir: page.dir_label.clone(),
                    path: page.path.clone(),
                    folder: page.is_dir,
                    current: marked && current == Some(page.page.id.as_str()),
                })
                .collect()
        };
        let listed = !index.pages.is_empty() || index.parent_path.is_some();
        SidebarView {
            index: listed.then(|| SidebarIndex {
                header: index.header.clone(),
                path: index.path.clone(),
                parent: index.parent_path.clone(),
                pages: lines(&index.pages, true),
            }),
            headings: headings
                .headings
                .iter()
                .map(|heading| SidebarHeading {
                    id: heading.id.clone(),
                    text: heading.text.clone(),
                    level: heading.level,
                })
                .collect(),
            recent: lines(&recent.pages, false),
        }
    }
}

/// A page that links to the shown page.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct BacklinkEntry {
    pub title: String,
    pub url: String,
    pub items: Vec<BacklinkLine>,
}

#[derive(Debug, Clone, PartialEq, Facet)]
pub struct BacklinkLine {
    pub text: String,
    pub url: String,
}

impl From<&BacklinkGroup> for BacklinkEntry {
    fn from(group: &BacklinkGroup) -> BacklinkEntry {
        BacklinkEntry {
            title: group.title.clone(),
            url: group.url.clone(),
            items: group
                .items
                .iter()
                .map(|item| BacklinkLine {
                    text: item.text.clone(),
                    url: item.url.clone(),
                })
                .collect(),
        }
    }
}
