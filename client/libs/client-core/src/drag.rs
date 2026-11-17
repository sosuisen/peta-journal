//! The rules of a drag of the mouse that starts on the text of an item, for every shell that has
//! a mouse. Inside the text the drag selects letters, and the shell's text does that. Once the
//! drag leaves the text up or down, it selects items: [`ItemDrag`] decides when, and which
//! events tell the core. While items are selected, the mouse in a band at the top or the
//! bottom of the list scrolls it by [`edge_step`].
//!
//! The places are in the list, from its top down. The shell measures them; the rules here do
//! not know its toolkit.

use crate::app::Event;

/// How far the mouse leaves the text of the item, up or down, before the drag selects items.
const LEAVE_MARGIN: f64 = 10.0;
/// The band inside each edge of the list where the mouse scrolls it.
const EDGE_BAND: f64 = 40.0;
/// The scroll of one step when the mouse is at the very edge, or beyond it.
const FASTEST_STEP: f64 = 24.0;
/// How long one step of the scroll takes.
pub const EDGE_STEP_MS: u32 = 16;

/// What the mouse of a drag did, as a shell tells it.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum DragMove {
    /// The mouse went down on the text of the item, which stands from `top` to `bottom`.
    Pressed {
        item_id: String,
        top: f64,
        bottom: f64,
    },
    /// The mouse moved to `y`, over the item `item_under`, or over no item. A shell also tells
    /// this after a step of the scroll, as the item under a still mouse changes.
    Moved { y: f64, item_under: Option<String> },
    /// The mouse button came up.
    Released,
}

/// A drag that starts on the text of an item.
#[derive(Debug, Default)]
pub struct ItemDrag {
    /// The item the mouse went down on, with the top and the bottom of its text.
    pressed: Option<(String, f64, f64)>,
    /// The item the selection was last told to reach. None while no items are selected.
    reached: Option<String>,
}

impl ItemDrag {
    /// The events that `step` of the mouse sends to the core: none while the drag stays in the
    /// text, and none when the selection already reaches the item under the mouse.
    pub fn events_of(&mut self, step: DragMove) -> Vec<Event> {
        match step {
            DragMove::Pressed {
                item_id,
                top,
                bottom,
            } => {
                self.pressed = Some((item_id, top, bottom));
                self.reached = None;
                vec![]
            }
            DragMove::Moved { y, item_under } => self.moved(y, item_under),
            DragMove::Released => {
                self.pressed = None;
                match self.reached.take() {
                    Some(_) => vec![Event::SelectionMouseUp],
                    None => vec![],
                }
            }
        }
    }

    fn moved(&mut self, y: f64, item_under: Option<String>) -> Vec<Event> {
        let Some((item_id, top, bottom)) = &self.pressed else {
            return vec![];
        };
        let mut events = vec![];
        if self.reached.is_none() {
            if !leaves_text(y, *top, *bottom) {
                return events;
            }
            events.push(Event::DragBeyondBounds {
                item_id: item_id.clone(),
            });
            self.reached = Some(item_id.clone());
        }
        if let Some(item_id) = item_under.filter(|id| self.reached.as_ref() != Some(id)) {
            events.push(Event::ItemMouseEnter {
                item_id: item_id.clone(),
            });
            self.reached = Some(item_id);
        }
        events
    }
}

/// The mouse at `y` has left the text from `top` to `bottom`, far enough that the drag selects
/// items.
pub fn leaves_text(y: f64, top: f64, bottom: f64) -> bool {
    y < top - LEAVE_MARGIN || y > bottom + LEAVE_MARGIN
}

/// How far the list scrolls in one step with the mouse at `y`, for a list that can be seen from
/// `top` to `bottom`. Below zero is up. Zero when the mouse is not in a band. The deeper the
/// mouse is in the band, the faster; beyond the edge is as fast as at it.
pub fn edge_step(y: f64, top: f64, bottom: f64) -> f64 {
    let depth = |inside: f64| (inside / EDGE_BAND).min(1.0) * FASTEST_STEP;
    if y < top + EDGE_BAND {
        -depth(top + EDGE_BAND - y)
    } else if y > bottom - EDGE_BAND {
        depth(y - (bottom - EDGE_BAND))
    } else {
        0.0
    }
}

/// Where to look for the item under the mouse at `y`, in a list that can be seen from `top` to
/// `bottom`. A place beyond it counts as the nearest place in it: the mouse of a drag may leave
/// the window. One inside the ends, where a row is and not the edge of one.
pub fn place_in_list(y: f64, top: f64, bottom: f64) -> f64 {
    y.max(top + 1.0).min(bottom - 1.0)
}
