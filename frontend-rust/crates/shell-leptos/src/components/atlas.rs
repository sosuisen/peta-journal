//! The canvas of an atlas page: the pages and folders of its directory as cards, drawn by
//! PixiJS in `atlas.js`. A card without a place waits in the list of unplaced cards beside
//! the canvas, and is placed by dragging it onto the canvas. A card on the canvas moves by
//! dragging it there.

use crate::core::CoreHandle;
use leptos::prelude::*;
use peta_core::app::Event;
use peta_core::screens::atlas::{CARD_HEIGHT, CARD_WIDTH};
use peta_core::view::AtlasCard;
use wasm_bindgen::prelude::*;

/// The drag data of an unplaced card. A type of its own keeps text dragged from elsewhere
/// from being taken as a card.
const CARD_MIME: &str = "application/x-peta-atlas-card";

/// An unplaced card being dragged: its page and where on the card it was grabbed, as a
/// share of the card's width and height. A share and not pixels, as a long label makes
/// the card in the list taller than on the canvas.
#[derive(serde::Serialize, serde::Deserialize)]
struct Grab {
    page_id: String,
    x: f64,
    y: f64,
}

// The global class of `atlas.js`, which index.html loads
#[wasm_bindgen]
extern "C" {
    type AtlasCanvas;

    #[wasm_bindgen(constructor)]
    fn new(
        host: &web_sys::HtmlElement,
        on_open: &js_sys::Function,
        on_place: &js_sys::Function,
    ) -> AtlasCanvas;

    #[wasm_bindgen(method, js_name = setCards)]
    fn set_cards(this: &AtlasCanvas, json: String);

    #[wasm_bindgen(method)]
    fn destroy(this: &AtlasCanvas);
}

/// The placed cards as `atlas.js` reads them.
fn placed_json(cards: &[AtlasCard]) -> String {
    let placed: Vec<_> = cards
        .iter()
        .filter_map(|c| {
            c.bounds.map(|b| {
                serde_json::json!({
                    "id": c.page_id,
                    "label": c.label,
                    "path": c.path,
                    "isDir": c.is_dir,
                    "x": b.x,
                    "y": b.y,
                    "width": b.width,
                    "height": b.height,
                })
            })
        })
        .collect();
    serde_json::Value::Array(placed).to_string()
}

#[component]
pub fn Atlas() -> impl IntoView {
    let core: CoreHandle = expect_context();
    let cards = core.select(|v| v.atlas_cards.clone());
    let unplaced = Memo::new(move |_| {
        cards.with(|cs| cs.iter().filter(|c| c.bounds.is_none()).cloned().collect::<Vec<_>>())
    });
    let host = NodeRef::<leptos::html::Div>::new();
    let canvas = StoredValue::new_local(None::<AtlasCanvas>);
    // A click on the label of a card on the canvas; kept as long as the canvas lives
    let on_open = StoredValue::new_local(Closure::<dyn Fn(String, bool, bool)>::new(
        move |path: String, new_tab: bool, new_window: bool| {
            core.send(Event::GoToPage { path, new_tab, new_window });
        },
    ));
    // A card dragged to a new place on the canvas
    let on_place = StoredValue::new_local(Closure::<dyn Fn(String, f64, f64)>::new(
        move |page_id: String, x: f64, y: f64| {
            core.send(Event::PlaceAtlasCard { page_id, x, y });
        },
    ));

    Effect::new(move |_| {
        let json = cards.with(|cs| placed_json(cs));
        let Some(el) = host.get() else {
            return;
        };
        canvas.update_value(|c| {
            c.get_or_insert_with(|| {
                on_open.with_value(|open| {
                    on_place.with_value(|place| {
                        AtlasCanvas::new(&el, open.as_ref().unchecked_ref(), place.as_ref().unchecked_ref())
                    })
                })
            })
            .set_cards(json);
        });
    });
    on_cleanup(move || {
        canvas.try_update_value(|c| {
            if let Some(c) = c.take() {
                c.destroy();
            }
        });
    });

    let on_drop = move |ev: web_sys::DragEvent| {
        ev.prevent_default();
        let Some(grab) = ev
            .data_transfer()
            .and_then(|dt| dt.get_data(CARD_MIME).ok())
            .and_then(|json| serde_json::from_str::<Grab>(&json).ok())
        else {
            return;
        };
        let Some(el) = host.get_untracked() else {
            return;
        };
        // The canvas measures its center in whole pixels from its own size, as here
        let rect = el.get_bounding_client_rect();
        let center_x = (el.client_width() as f64 / 2.0).round();
        let center_y = (el.client_height() as f64 / 2.0).round();
        // The grabbed spot of the card stays under the pointer, as the dragged image shows it.
        // The card is placed by its center, which lies this far from the grabbed spot.
        core.send(Event::PlaceAtlasCard {
            page_id: grab.page_id,
            x: ev.client_x() as f64 - rect.left() - center_x + (0.5 - grab.x) * CARD_WIDTH,
            y: ev.client_y() as f64 - rect.top() - center_y + (0.5 - grab.y) * CARD_HEIGHT,
        });
    };

    view! {
        <div class="atlas">
            <div class="atlas-unplaced">
                <For
                    each=move || unplaced.get()
                    key=|c| (c.page_id.clone(), c.label.clone(), c.is_dir)
                    let:card
                >
                    {
                        let page_id = card.page_id.clone();
                        let path = card.path.clone();
                        view! {
                            <div class="atlas-card" class:folder=card.is_dir draggable="true"
                                on:dragstart=move |ev: web_sys::DragEvent| {
                                    // The card, not the label inside it that may be under the pointer.
                                    // Not `current_target`: Leptos hands the event on from an element
                                    // further up, which would measure the grab against that element.
                                    let Some(el) = ev
                                        .target()
                                        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                                        .and_then(|t| t.closest(".atlas-card").ok().flatten())
                                    else {
                                        return;
                                    };
                                    let rect = el.get_bounding_client_rect();
                                    let grab = Grab {
                                        page_id: page_id.clone(),
                                        x: ((ev.client_x() as f64 - rect.left()) / rect.width()).clamp(0.0, 1.0),
                                        y: ((ev.client_y() as f64 - rect.top()) / rect.height()).clamp(0.0, 1.0),
                                    };
                                    if let (Some(dt), Ok(json)) = (ev.data_transfer(), serde_json::to_string(&grab)) {
                                        let _ = dt.set_data(CARD_MIME, &json);
                                        dt.set_effect_allowed("move");
                                    }
                                }>
                                <span class="atlas-card-label" on:click=move |ev: web_sys::MouseEvent| {
                                    core.send(Event::GoToPage {
                                        path: path.clone(),
                                        new_tab: ev.ctrl_key() || ev.meta_key(),
                                        new_window: ev.shift_key(),
                                    });
                                }>{card.label.clone()}</span>
                            </div>
                        }
                    }
                </For>
            </div>
            <div class="atlas-canvas" node_ref=host
                on:dragover=move |ev: web_sys::DragEvent| ev.prevent_default()
                on:drop=on_drop></div>
        </div>
    }
}
