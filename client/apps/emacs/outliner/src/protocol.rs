//! The JSON lines this process and `peta-journal.el` send each other.

use client_core::view::{BacklinkGroup, PageProperty, Row, SearchEntry, TaskState};
use serde::{Deserialize, Serialize};

/// One line from Emacs. Emacs numbers its lines, so it can tell which of them an answer has seen.
#[derive(Debug, PartialEq, Deserialize)]
pub struct Envelope {
    pub seq: u64,
    #[serde(flatten)]
    pub message: Inbound,
}

/// What the user did in Emacs.
#[derive(Debug, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Inbound {
    Start {
        url: String,
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

/// Which page of a date to open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DateUnit {
    Day,
    Week,
    Month,
    Year,
}

/// What Emacs is told to do.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Outbound {
    /// Which line of Emacs was handled last, and whether the core holds keys for an editor that
    /// is not there yet. Emacs waits on this after a key that changes the outline.
    State {
        ack: u64,
        holding: bool,
    },
    Rows {
        ack: u64,
        page_id: Option<String>,
        rows: Vec<RowView>,
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
    Backlinks {
        groups: Vec<BacklinkEntry>,
    },
    Message {
        text: String,
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

/// One row of the outline, as Emacs draws it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RowView {
    pub id: String,
    pub level: i32,
    pub body: String,
    pub bullet: Bullet,
    /// The state of the task, which Emacs marks behind the bullet. None when the item is
    /// not a task.
    pub task: Option<TaskMark>,
    pub collapsed: bool,
    /// The date above the first item of a journal day.
    pub header: Option<String>,
}

impl RowView {
    pub fn of(row: &Row<()>) -> RowView {
        RowView {
            id: row.item.id.clone(),
            level: row.item.level,
            body: row.item.body.clone(),
            bullet: Bullet::of(row.is_broken_transclusion, row.is_transclusion),
            task: TaskMark::of(row.task),
            collapsed: row.is_collapsed && row.has_children,
            header: row.date_header.clone(),
        }
    }
}

/// What the mark at the start of a row shows. A task has the bullet of its kind too: its
/// state is a mark of its own, as in the web frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Bullet {
    Plain,
    Transclusion,
    Broken,
}

impl Bullet {
    pub fn of(broken: bool, transclusion: bool) -> Bullet {
        if broken {
            Bullet::Broken
        } else if transclusion {
            Bullet::Transclusion
        } else {
            Bullet::Plain
        }
    }
}

/// The state of a task, for the mark behind the bullet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
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
#[derive(Debug, Clone, PartialEq, Serialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FreeKey {
    pub key: String,
    pub label: String,
}

/// A page that links to the shown page.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BacklinkEntry {
    pub title: String,
    pub url: String,
    pub items: Vec<BacklinkLine>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
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
