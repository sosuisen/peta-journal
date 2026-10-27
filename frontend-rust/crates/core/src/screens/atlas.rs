//! The canvas of an atlas page: the pages and folders of its directory as cards. A card lies
//! where the `atlas_bounds` property of its page says, in coordinates whose origin is the
//! center of the canvas. A page without the property waits in the list of unplaced cards.

use crate::app::{apply, done, run, Cmd, Model};
use crate::effects::UiOp;
use crate::model::Page;
use crate::screens::sidebar;

/// The page property that holds where a card lies: `x,y,width,height`.
pub const BOUNDS_KEY: &str = "atlas_bounds";

/// The size a card takes when it is first placed.
pub const CARD_WIDTH: f64 = 120.0;
pub const CARD_HEIGHT: f64 = 40.0;

/// The place and size of a card. `x` and `y` are its top left corner; `y` grows downward,
/// as on the screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Bounds {
    /// Reads a bounds value as the server writes it. A value that is not four numbers
    /// counts as no value, so the card stays in the list and can be placed again.
    pub fn parse(value: &str) -> Option<Bounds> {
        let numbers = value
            .split(',')
            .map(|n| n.trim().parse::<f64>().ok().filter(|n| n.is_finite()))
            .collect::<Option<Vec<f64>>>()?;
        match numbers[..] {
            [x, y, width, height] => Some(Bounds { x, y, width, height }),
            _ => None,
        }
    }

    pub fn of(page: &Page) -> Option<Bounds> {
        page.properties
            .get(BOUNDS_KEY)
            .and_then(|v| v.as_str())
            .and_then(Bounds::parse)
    }

    fn to_value(self) -> String {
        format!("{},{},{},{}", self.x, self.y, self.width, self.height)
    }
}

/// Places the card of a page with its top left corner at the given point of the canvas.
/// The shell knows where the card was grabbed, so it gives the corner rather than the drop
/// point. The card moves at once; a refusal of the server puts it back.
pub fn place_card(model: &mut Model, page_id: String, x: f64, y: f64) -> Cmd {
    let Some(page) = model
        .sidebar
        .index_pages
        .iter_mut()
        .find(|p| p.id == page_id)
    else {
        return done();
    };
    // A card keeps its size when it moves; whole pixels keep the stored value short
    let (width, height) = Bounds::of(page)
        .map(|b| (b.width, b.height))
        .unwrap_or((CARD_WIDTH, CARD_HEIGHT));
    let bounds = Bounds {
        x: x.round(),
        y: y.round(),
        width,
        height,
    };
    let before = page.properties.clone();
    page.properties
        .insert(BOUNDS_KEY.to_string(), bounds.to_value().into());
    run(move |api| async move {
        match api
            .upsert_page_property(&page_id, BOUNDS_KEY, "bounds", &bounds.to_value())
            .await
        {
            Ok(updated) => apply(&api, "atlas card placed", move |m| {
                if let Some(page) = index_page_mut(m, &updated.id) {
                    page.properties = updated.properties;
                }
                done()
            }),
            Err(e) => {
                api.notify(UiOp::Alert(e.0));
                apply(&api, "atlas card put back", move |m| {
                    if let Some(page) = index_page_mut(m, &page_id) {
                        page.properties = before;
                    }
                    done()
                });
            }
        }
    })
}

fn index_page_mut<'a>(model: &'a mut Model, page_id: &str) -> Option<&'a mut Page> {
    model.sidebar.index_pages.iter_mut().find(|p| p.id == page_id)
}

/// The cards of the shown atlas page, in the order of the sidebar's index list.
pub fn cards(model: &Model) -> Vec<(Page, Option<Bounds>)> {
    let shows_atlas = super::page::page_state(model)
        .and_then(|ps| ps.page.as_ref())
        .is_some_and(super::page::is_atlas);
    if !shows_atlas {
        return Vec::new();
    }
    sidebar::sorted_index_pages(&model.sidebar.index_pages, sidebar::index_sort(model))
        .into_iter()
        .map(|p| {
            let bounds = Bounds::of(&p);
            (p, bounds)
        })
        .collect()
}
