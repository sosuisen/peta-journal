//! The date of today, as the clock of the shell has it (#796). The core asks the clock again
//! and again, so that the journal of today follows a new day. A shell may also tell the core
//! to look at once, as when the machine woke.

use crate::app::{run, Cmd, Event, Model, Screen};
use crate::effects::TimerOutput;
use crate::screens::route;
use crux_core::Command;

/// How long the core waits between two looks at the clock. A wait until midnight would be
/// late after the machine slept, as a timer does not count the time asleep.
pub const CHECK_MS: u32 = 60_000;

/// Looks at the clock after [`CHECK_MS`].
pub fn watch(model: &mut Model) -> Cmd {
    let timer_id = model.next_timer_id();
    run(move |api| async move {
        if api.sleep(timer_id, CHECK_MS).await != TimerOutput::Fired {
            return;
        }
        let (now, _) = api.local_now().await;
        api.send_event(Event::DateChecked {
            today: now.day_name(),
        });
    })
}

/// The clock says `today` after the wait: the core sees the date, and looks again later.
pub fn checked(model: &mut Model, today: String) -> Cmd {
    let next = watch(model);
    Command::all([next, seen(model, today)])
}

/// Looks at the clock now. The wait of [`watch`] goes on: a second one would look twice as
/// often.
pub fn look() -> Cmd {
    run(|api| async move {
        let (now, _) = api.local_now().await;
        api.send_event(Event::DateSeen {
            today: now.day_name(),
        });
    })
}

/// The clock says `today`: the journal of today opens the new day when the date changed and
/// no item is being edited. A shell that started the core with `opens_new_day` false keeps
/// the old day (#1307).
pub fn seen(model: &mut Model, today: String) -> Cmd {
    model.today = today;
    let old_day = matches!(
        &model.screen,
        Screen::Journal(j) if j.page_name == "today" && j.current_date != model.today
    );
    // Opening a page closes the editor, and the text typed would be cut short. The next look
    // at the clock tries again
    let editing = !model.state.editing_item_ids().is_empty();
    if !model.opens_new_day || !old_day || editing {
        return Command::done();
    }
    route::open(model, "today", None)
}
