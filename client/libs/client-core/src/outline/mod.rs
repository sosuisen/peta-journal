pub mod selection;
pub mod state;
pub mod tree;
pub mod undo;

pub use selection::Selection;
pub use state::{FocusOrder, FocusRequest, Heading, OutlineState};
pub use undo::{CursorState, UndoEntry, UndoOp, UndoStacks};

use crate::model::DeferredKey;

/// Carries key presses from one editor to the next. Enter shows the next item at once, but
/// its editor gets the focus only after the next render; keys pressed in between would go
/// to the old editor (a Tab would indent the old item). While the relay holds, the shell
/// hands the old editor's keys here instead, and plays them in the editor it focuses.
#[derive(Debug, Default)]
pub struct KeyRelay {
    held: Option<Vec<DeferredKey>>,
    /// The orders to focus an editor that the shell has not answered yet.
    unanswered: usize,
}

impl KeyRelay {
    /// Starts holding keys until the next editor is focused. Keys already held stay.
    pub fn hold_until_focus(&mut self) {
        self.held.get_or_insert_with(Vec::new);
    }

    pub fn is_holding(&self) -> bool {
        self.held.is_some()
    }

    /// Keeps a key press for the next focused editor; does nothing when not holding.
    pub fn hold(&mut self, key: DeferredKey) {
        if let Some(held) = &mut self.held {
            held.push(key);
        }
    }

    /// Returns the keys held so far, for the order to focus the next editor. It goes on holding
    /// until the field of that editor takes the keyboard (ADR 087).
    pub fn hand_over(&mut self) -> Vec<DeferredKey> {
        self.unanswered += 1;
        self.held.as_mut().map(std::mem::take).unwrap_or_default()
    }

    /// The shell answered an order to focus an editor. True when no later order waits for its
    /// answer: the keys are for the editor of the last order only.
    pub fn answered(&mut self) -> bool {
        self.unanswered = self.unanswered.saturating_sub(1);
        self.unanswered == 0
    }

    /// Stops holding and returns the keys in the order they were pressed.
    pub fn release(&mut self) -> Vec<DeferredKey> {
        self.unanswered = 0;
        self.held.take().unwrap_or_default()
    }
}
