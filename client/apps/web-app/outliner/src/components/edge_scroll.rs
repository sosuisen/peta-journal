//! The scrolling of the outline while a drag selection holds the mouse at the top or the
//! bottom edge of the viewport: the rows beyond the edge cannot be reached otherwise.

use crate::core::CoreHandle;
use crate::dom;
use client_core::Event;
use leptos::prelude::*;

/// The band inside each edge of the viewport where the mouse scrolls it.
const EDGE_PX: f64 = 40.0;
/// The scroll per frame when the mouse is at the very edge; less further inside the band.
const MAX_STEP_PX: f64 = 24.0;
const FRAME_MS: u32 = 16;

/// Scrolls `viewport` while the mouse stays in its edge bands. `Copy`, so the listeners of
/// the selection mode can share it.
#[derive(Clone, Copy)]
pub struct EdgeScroll {
    core: CoreHandle,
    viewport: NodeRef<leptos::html::Custom<&'static str>>,
    /// Pixels per frame, negative upward; zero when the mouse is outside the bands.
    step: StoredValue<f64>,
    /// Where the mouse is, so each frame can tell which row scrolled under it.
    pointer: StoredValue<(f64, f64)>,
    /// The row last reported under the mouse, so a frame that changes nothing stays silent.
    reported: StoredValue<Option<String>>,
    running: StoredValue<bool>,
}

impl EdgeScroll {
    pub fn new(
        core: CoreHandle,
        viewport: NodeRef<leptos::html::Custom<&'static str>>,
    ) -> EdgeScroll {
        EdgeScroll {
            core,
            viewport,
            step: StoredValue::new(0.0),
            pointer: StoredValue::new((0.0, 0.0)),
            reported: StoredValue::new(None),
            running: StoredValue::new(false),
        }
    }

    /// The mouse moved with the button held: picks the scroll speed from how deep in an edge
    /// band it is and keeps the frames going while there is a speed.
    pub fn follow(self, client_x: f64, client_y: f64) {
        let Some(vp) = self.viewport.get_untracked() else {
            return;
        };
        self.pointer.set_value((client_x, client_y));
        let rect = vp.get_bounding_client_rect();
        let step = if client_y < rect.top() + EDGE_PX {
            -depth(rect.top() + EDGE_PX - client_y)
        } else if client_y > rect.bottom() - EDGE_PX {
            depth(client_y - (rect.bottom() - EDGE_PX))
        } else {
            0.0
        };
        self.step.set_value(step);
        if step != 0.0 && !self.running.get_value() {
            self.running.set_value(true);
            self.frame();
        }
    }

    /// Stops at once, e.g. when the button is released or the selection mode ends.
    pub fn stop(self) {
        self.step.try_set_value(0.0);
    }

    fn frame(self) {
        // The outline may be gone by the time the frame fires
        let step = self.step.try_get_value().unwrap_or(0.0);
        let vp = self.viewport.try_get_untracked().flatten();
        let (Some(vp), false) = (vp, step == 0.0) else {
            self.running.try_set_value(false);
            return;
        };
        vp.set_scroll_top((vp.scroll_top() as f64 + step).max(0.0) as i32);
        self.report_row_under_mouse();
        dom::set_timeout(FRAME_MS, move || self.frame());
    }

    /// Extends the selection to the row now under the still mouse. The browser fires no
    /// mouseenter for rows that scroll under a mouse that does not move.
    fn report_row_under_mouse(self) {
        let (x, y) = self.pointer.get_value();
        let row_id = dom::document()
            .element_from_point(x as f32, y as f32)
            .and_then(|el| el.closest(".outline-row").ok().flatten())
            .and_then(|row| row.get_attribute("data-row-id"));
        let Some(row_id) = row_id else {
            return;
        };
        if self.reported.get_value().as_deref() == Some(row_id.as_str()) {
            return;
        }
        self.reported.set_value(Some(row_id.clone()));
        self.core.send(Event::ItemMouseEnter { item_id: row_id });
    }
}

/// The step for a mouse `inside` pixels into a band: the deeper, the faster, capped at the edge.
fn depth(inside: f64) -> f64 {
    (inside / EDGE_PX).min(1.0) * MAX_STEP_PX
}
