//! The clock that dates what the journal writes: the items, the pages, the property
//! definitions and the IDs.
//!
//! Two devices that sync take the newer of two dates, so the dates decide what stays. The app
//! around the core gives the clock (`CoreHost`). An app gives [`SystemClock`]. A test gives
//! [`SteppedClock`] and moves it on between two changes, where it had to wait for the system
//! clock before.
//!
//! The times of the files are not from this clock: the file system dates them.

use std::fmt::Debug;
use std::sync::atomic::{AtomicI64, Ordering};

pub trait Clock: Debug + Send + Sync {
    /// The time now, in milliseconds since 1970 (UTC).
    fn now_millis(&self) -> i64;
}

/// The clock of the system.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_millis(&self) -> i64 {
        chrono::Utc::now().timestamp_millis()
    }
}

/// The clock of the system, moved ahead by what a test adds. It never stands still, so a
/// date it gives is not before the date of a file written earlier.
#[derive(Debug, Default)]
pub struct SteppedClock {
    ahead: AtomicI64,
}

impl SteppedClock {
    /// Moves the clock ahead by the milliseconds given, as time that passed.
    pub fn advance(&self, millis: i64) {
        self.ahead.fetch_add(millis, Ordering::SeqCst);
    }
}

impl Clock for SteppedClock {
    fn now_millis(&self) -> i64 {
        SystemClock.now_millis() + self.ahead.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 進めた時計は、進めた分だけシステムの時計より先の時刻を返す。
    #[test]
    fn a_stepped_clock_is_ahead_of_the_system_clock_by_what_was_added() {
        let clock = SteppedClock::default();
        let before = SystemClock.now_millis();
        assert!(clock.now_millis() >= before);

        clock.advance(60_000);
        clock.advance(40_000);

        let now = clock.now_millis();
        assert!(now >= before + 100_000, "{now} {before}");
        assert!(now <= SystemClock.now_millis() + 100_000);
    }
}
