//! What an extension function is told and what it asks for (see ADR 009), and the runtime
//! that runs its script.
//!
//! The runtime is QuickJS, behind the `extensions` feature (ADR 029). A build without the
//! feature has a runtime with the same methods that runs no script. So the code that calls the
//! extensions is the same in both builds.

use std::time::Duration;

use hmtid::Hmtid;
use journal_names::JournalName;
use serde::Serialize;

use crate::domain::model::ItemPosition;

#[cfg(feature = "extensions")]
mod quickjs;
#[cfg(feature = "extensions")]
pub use quickjs::ExtensionRuntime;

#[cfg(not(feature = "extensions"))]
mod none;
#[cfg(not(feature = "extensions"))]
pub use none::ExtensionRuntime;

/// The page an event is about, as the script sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub id: String,
    pub file_name: String,
    pub dir_path: String,
    pub page_type: String,
    /// The period of a journal, read from its name by `journal-names` as the server reads it,
    /// so a script does not read the name its own way. `null` for a page that is not a journal.
    pub journal: Option<JournalPeriod>,
}

/// The period a journal page covers, as the script sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalPeriod {
    /// `daily`, `weekly`, `monthly` or `yearly`.
    pub period: &'static str,
    /// The English name of the day of a daily journal, such as `Monday`.
    pub weekday: Option<&'static str>,
}

const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

impl From<JournalName> for JournalPeriod {
    fn from(name: JournalName) -> Self {
        let (period, weekday) = match name {
            JournalName::Daily(date) => ("daily", Some(WEEKDAYS[date.weekday() as usize])),
            JournalName::Weekly { .. } => ("weekly", None),
            JournalName::Monthly { .. } => ("monthly", None),
            JournalName::Yearly(_) => ("yearly", None),
        };
        JournalPeriod { period, weekday }
    }
}

/// What happened, which the script may answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionEvent {
    PageCreate(PageInfo),
    /// Tags were added to the tags property of the page.
    PageTagAdd(PageInfo, Vec<String>),
}

/// An item the script asked for. Its id is given out at once, so the script can place the
/// next item after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemRequest {
    pub id: Hmtid,
    pub page_id: String,
    pub position: ItemPosition,
    pub level: i32,
    pub body: String,
    pub transclude: Option<String>,
}

/// How long a run may take and how much memory it may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtensionLimits {
    pub time: Duration,
    /// In bytes.
    pub memory: usize,
}

impl Default for ExtensionLimits {
    /// A run blocks the creation of a page, so it gets little time. A template takes a few
    /// milliseconds.
    fn default() -> Self {
        ExtensionLimits {
            time: Duration::from_secs(2),
            memory: 32 * 1024 * 1024,
        }
    }
}

/// Asks the user a yes-or-no question for a script, and waits for the answer.
pub trait Confirmer: Send + Sync {
    fn confirm(&self, message: &str) -> bool;
}
