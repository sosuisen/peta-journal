//! The names of the journal pages, and the weeks they are named after. The journal and the
//! frontend core read a name and number a week with this one crate, so a week has one name
//! on both sides.
//!
//! The crate has no dependency: the frontend core is also built for wasm, and it is told the
//! date by its shell, so it has no use for a clock.

mod date;
mod name;
mod week;

pub use date::{days_in_month, is_leap_year, Date};
pub use name::{is_journal_name, JournalName, NotAJournalName};
pub use week::{first_day_of_week, week_of, weeks_in_year, weeks_of_month, WeekNumbering};
