pub mod launcher;
pub mod pending_delete_prompt;
pub mod watch;

use journal_core::clock::{Clock, SystemClock};

/// The time now by the clock of the system, in milliseconds since 1970. The server times its
/// own waiting with it: the quiet time of a file and the pause between two prompts. Nothing
/// of it is written to the journal, which has a clock of its own.
pub(crate) fn now_millis() -> i64 {
    SystemClock.now_millis()
}
