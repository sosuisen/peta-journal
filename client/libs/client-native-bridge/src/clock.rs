//! The time this side reads and waits for (ADR 036).

use chrono::{Datelike, Local, Timelike};
use client_core::effects::LocalDateTimeDto;
use std::thread;
use std::time::Duration;

/// The time of this side. It is not a part of [`Host`](crate::Host): a host is a way to the
/// journal, and every way reads the same clock.
pub trait Clock: Send + Sync + 'static {
    /// The local date and time, and the milliseconds since 1970.
    fn now(&self) -> (LocalDateTimeDto, u64);
    /// Returns after this many milliseconds.
    fn sleep(&self, ms: u32);
}

/// The clock of this machine.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> (LocalDateTimeDto, u64) {
        let now = Local::now();
        let local = LocalDateTimeDto {
            year: now.year(),
            month: now.month(),
            day: now.day(),
            hour: now.hour(),
            minute: now.minute(),
        };
        (local, now.timestamp_millis().unsigned_abs())
    }

    fn sleep(&self, ms: u32) {
        thread::sleep(Duration::from_millis(ms.into()));
    }
}
