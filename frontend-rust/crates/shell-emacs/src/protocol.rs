//! The JSON lines this process and `peta-journal.el` send each other.

use peta_core::view::{BacklinkGroup, Row, TaskState};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    GoToPage {
        path: String,
    },
    OpenWikiLink {
        text: String,
    },
    OpenTag {
        tag: String,
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
        index: usize,
    },
    Answer {
        req: u64,
        value: Value,
    },
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
    Status {
        connection: String,
    },
    Search {
        query: String,
        results: Vec<SearchEntry>,
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
    /// A question for the user. Emacs answers with [`Inbound::Answer`].
    Ask {
        req: u64,
        kind: AskKind,
        message: String,
    },
    Copy {
        text: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AskKind {
    Confirm,
    ConfirmDestructive,
    Prompt,
    Clipboard,
}

/// One row of the outline, as Emacs draws it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RowView {
    pub id: String,
    pub level: i32,
    pub body: String,
    pub bullet: Bullet,
    pub collapsed: bool,
    /// The date above the first item of a journal day.
    pub header: Option<String>,
}

impl RowView {
    pub fn of(row: &Row) -> RowView {
        RowView {
            id: row.item.id.clone(),
            level: row.item.level,
            body: row.item.body.clone(),
            bullet: Bullet::of(row.is_broken_transclusion, row.is_transclusion, row.task),
            collapsed: row.is_collapsed && row.has_children,
            header: row.date_header.clone(),
        }
    }
}

/// What the mark at the start of a row shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Bullet {
    Plain,
    Task,
    Running,
    Done,
    Transclusion,
    Broken,
}

impl Bullet {
    pub fn of(broken: bool, transclusion: bool, task: Option<TaskState>) -> Bullet {
        if broken {
            return Bullet::Broken;
        }
        if transclusion {
            return Bullet::Transclusion;
        }
        match task {
            Some(TaskState::NotStarted) => Bullet::Task,
            Some(TaskState::Running) => Bullet::Running,
            Some(TaskState::Done) => Bullet::Done,
            None => Bullet::Plain,
        }
    }
}

/// One line of the search results. `index` is what [`Inbound::SearchResult`] sends back.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SearchEntry {
    pub index: usize,
    pub kind: SearchKind,
    pub title: String,
    pub snippet: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchKind {
    Page,
    Create,
    Item,
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
