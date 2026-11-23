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

    /// Gives each question the number the shell would give its request, from 1, and keeps
    /// what the shell was given.
    #[derive(Default)]
    struct Screen {
        given: Vec<(u32, &'static str)>,
    }

    impl Screen {
        fn give(&mut self) -> impl FnOnce(&'static str) -> u32 + '_ {
            |question| {
                let id = self.given.len() as u32 + 1;
                self.given.push((id, question));
                id
            }
        }
    }

    // 何も聞いていないときの質問は、すぐにシェルへ渡る。
    #[test]
    fn a_question_goes_to_the_shell_at_once_when_none_is_on_the_screen() {
        let (mut questions, mut screen) = (Questions::default(), Screen::default());

        questions.ask("delete?", screen.give());

        assert_eq!(screen.given, [(1, "delete?")]);
    }

    // 質問がシェルにある間の質問は、その答えを待つ。
    #[test]
    fn a_question_waits_for_the_answer_to_the_one_on_the_screen() {
        let (mut questions, mut screen) = (Questions::default(), Screen::default());
        questions.ask("delete?", screen.give());

        questions.ask("rename?", screen.give());
        questions.ask("move?", screen.give());
        assert_eq!(screen.given, [(1, "delete?")]);

        questions.answered(1, screen.give());
        assert_eq!(screen.given, [(1, "delete?"), (2, "rename?")]);
        questions.answered(2, screen.give());
        assert_eq!(screen.given, [(1, "delete?"), (2, "rename?"), (3, "move?")]);
    }

    // 質問でない要求の答えでは、次の質問は渡らない。
    #[test]
    fn the_answer_to_a_request_that_is_not_the_question_gives_no_next_one() {
        let (mut questions, mut screen) = (Questions::default(), Screen::default());
        questions.ask("delete?", screen.give());
        questions.ask("rename?", screen.give());

        // The answer to a request for the clipboard, say
        questions.answered(9, screen.give());

        assert_eq!(screen.given, [(1, "delete?")]);
    }

    // すべての質問に答えたあとの質問は、またすぐにシェルへ渡る。
    #[test]
    fn a_question_after_all_are_answered_goes_to_the_shell_at_once() {
        let (mut questions, mut screen) = (Questions::default(), Screen::default());
        questions.ask("delete?", screen.give());
        questions.answered(1, screen.give());

        questions.ask("rename?", screen.give());

        assert_eq!(screen.given, [(1, "delete?"), (2, "rename?")]);
    }

    // 確認と破壊的な確認と入力の求めだけが、質問である。
    #[test]
    fn only_a_confirm_a_destructive_confirm_and_a_prompt_are_questions() {
        let questions = [
            UiOp::Confirm("sure?".into()),
            UiOp::ConfirmDestructive("delete?".into()),
            UiOp::Prompt("name?".into()),
        ];
        assert!(questions.iter().all(is_question));

        let others = [
            UiOp::Alert("done".into()),
            UiOp::SetTitle("notes".into()),
            UiOp::ReadClipboardText,
            UiOp::LocalNow,
        ];
        assert!(!others.iter().any(is_question));
    }
}
