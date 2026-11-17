//! The name of a journal page: a day, a week, a month or a year. The rules are in the crate
//! `journal-names`, which the frontend core reads the names with too.

pub use journal_names::{JournalName, NotAJournalName};

/// Whether a file name is the name of a journal.
pub fn is_journal_file_name(file_name: &str) -> bool {
    journal_names::is_journal_name(file_name)
}
