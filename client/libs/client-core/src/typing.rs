//! What a text field of a native shell tells when the user changes the Markdown of an item. The
//! shell hands the text and the cursor as its toolkit counts it, in UTF-16 units. The bridge
//! adds the time from its clock, and [`event_of`] makes the event of the core: the shells do not
//! read the clock, and do not count characters, each in its own way.

use crate::app::Event;
use crate::util::utf16::chars_of_utf16;

/// A change of the Markdown of an item in its text field.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum Typing {
    /// The user typed into the item, and the cursor is `cursor` UTF-16 units into `body`. The
    /// core closes an opening just typed (ADR 057).
    Typed {
        item_id: String,
        body: String,
        cursor: u64,
    },
    /// The Markdown of the item is `body` now, without a cursor to go by, as after a cut from a
    /// menu.
    Changed { item_id: String, body: String },
}

/// The event of the core for `typing` at the time `now_ms`. The core saves the body a moment
/// after the last change, and counts that moment from this time.
pub fn event_of(typing: Typing, now_ms: u64) -> Event {
    match typing {
        Typing::Typed {
            item_id,
            body,
            cursor,
        } => {
            let units = usize::try_from(cursor).unwrap_or(usize::MAX);
            Event::Typed {
                cursor: chars_of_utf16(&body, units),
                item_id,
                body,
                now_ms,
            }
        }
        Typing::Changed { item_id, body } => Event::BodyChanged {
            item_id,
            body,
            now_ms,
        },
    }
}
