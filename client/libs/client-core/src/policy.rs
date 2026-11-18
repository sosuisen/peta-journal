//! How a shell performs the effects of the core, apart from what it performs them with
//! (ADR 052).
//!
//! The rules are the same for every shell:
//!
//! - The requests to the journal go one at a time, in the order the core made them
//!   ([`InOrder`]). Sent side by side, a join reached the journal before the save of the text
//!   just typed, and the text was lost.
//! - The questions to the user go one at a time too ([`InOrder`]): a dialog on the screen is in
//!   the way of everything else, and so is a second one above it.
//! - A timer is known by the ID the core gave it, and is answered once: as fired or as
//!   cancelled, whichever comes first ([`Timers`]).
//!
//! What differs is the means: a thread, a task of the browser, or the lines to and from Emacs.
//! So the types here start nothing and wait for nothing. They only say what is next, and the
//! shell does it with its own means. They hold no lock either: a shell with threads keeps them
//! behind its own.

use std::collections::{HashMap, VecDeque};

/// What is done one at a time, in the order it came.
///
/// The shell calls [`InOrder::push`] for each new one. On `true` nothing was being done, and
/// the shell starts the work: it takes one with [`InOrder::take_next`], does it, and takes the next,
/// until there is none.
#[derive(Debug)]
pub struct InOrder<T> {
    waiting: VecDeque<T>,
    /// Whether something is being done now, so that what comes waits for it
    busy: bool,
}

impl<T> Default for InOrder<T> {
    fn default() -> InOrder<T> {
        InOrder::new()
    }
}

impl<T> InOrder<T> {
    pub const fn new() -> InOrder<T> {
        InOrder {
            waiting: VecDeque::new(),
            busy: false,
        }
    }

    /// Takes one more in. True when nothing is being done: the caller starts the work, and
    /// takes with `take_next` until nothing is left.
    #[must_use = "on true the caller starts the work"]
    pub fn push(&mut self, item: T) -> bool {
        self.waiting.push_back(item);
        !std::mem::replace(&mut self.busy, true)
    }

    /// The next to do, once the one before is done. With None the work is over, and the next
    /// `push` starts it again.
    pub fn take_next(&mut self) -> Option<T> {
        let next = self.waiting.pop_front();
        self.busy = next.is_some();
        next
    }

    /// True when nothing is being done and nothing waits.
    pub fn is_idle(&self) -> bool {
        !self.busy
    }
}

/// The timers that run, by the ID the core gave each. `T` is what the shell keeps of a timer:
/// the request to answer, and what stops its clock if the shell has such a thing.
///
/// A timer ends once. The shell takes it out with [`Timers::end`] when its time is over and
/// when the core cancels it; the second of the two finds nothing and does nothing.
#[derive(Debug)]
pub struct Timers<T> {
    running: HashMap<u64, T>,
}

impl<T> Default for Timers<T> {
    fn default() -> Timers<T> {
        Timers {
            running: HashMap::new(),
        }
    }
}

impl<T> Timers<T> {
    /// Keeps a timer that the shell has started.
    pub fn start(&mut self, id: u64, timer: T) {
        self.running.insert(id, timer);
    }

    /// Takes the timer out, to answer it. None when it has ended already.
    pub fn end(&mut self, id: u64) -> Option<T> {
        self.running.remove(&id)
    }

    /// True when no timer runs.
    pub fn is_empty(&self) -> bool {
        self.running.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 何もしていないときに来たものは、すぐに始まる。している間に来たものは、来た順に待つ。
    #[test]
    fn the_first_starts_the_work_and_the_others_wait_in_the_order_they_came() {
        let mut requests = InOrder::new();

        assert!(requests.push("save"));
        assert_eq!(requests.take_next(), Some("save"));
        // While the save is on its way
        assert!(!requests.push("merge"));
        assert!(!requests.push("load"));

        assert_eq!(requests.take_next(), Some("merge"));
        assert_eq!(requests.take_next(), Some("load"));
        assert_eq!(requests.take_next(), None);
    }

    // すべて終わったあとに来たものは、また始める。
    #[test]
    fn what_comes_after_all_is_done_starts_the_work_again() {
        let mut requests = InOrder::new();
        assert!(requests.push(1));
        assert_eq!(requests.take_next(), Some(1));
        assert_eq!(requests.take_next(), None);

        assert!(requests.push(2));
        assert_eq!(requests.take_next(), Some(2));
    }

    // 始めた者が最初の1つを取る前に来たものは、二重には始めない。
    #[test]
    fn what_comes_before_the_first_is_taken_does_not_start_the_work_twice() {
        let mut requests = InOrder::new();
        assert!(requests.push(1));
        assert!(!requests.push(2));

        assert_eq!(requests.take_next(), Some(1));
        assert_eq!(requests.take_next(), Some(2));
        assert_eq!(requests.take_next(), None);
    }

    // している間と待つものがある間は、暇ではない。すべて終わると暇になる。
    #[test]
    fn it_is_idle_only_when_nothing_is_done_and_nothing_waits() {
        let mut requests = InOrder::new();
        assert!(requests.is_idle());

        assert!(requests.push(1));
        assert!(!requests.is_idle());
        assert_eq!(requests.take_next(), Some(1));
        assert!(!requests.is_idle());

        assert_eq!(requests.take_next(), None);
        assert!(requests.is_idle());
    }

    // タイマーは、1回だけ終わる。時間切れと取り消しの後のほうは、何も見つけない。
    #[test]
    fn a_timer_ends_once_and_the_second_end_finds_nothing() {
        let mut timers = Timers::default();
        assert!(timers.is_empty());
        timers.start(7, "save");
        timers.start(8, "search");
        assert!(!timers.is_empty());

        assert_eq!(timers.end(7), Some("save"));
        assert_eq!(timers.end(7), None);
        assert_eq!(timers.end(9), None);
        assert!(!timers.is_empty());

        assert_eq!(timers.end(8), Some("search"));
        assert!(timers.is_empty());
    }
}
