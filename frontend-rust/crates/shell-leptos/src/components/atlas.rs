//! The canvas of an atlas page: the pages and folders of its directory as cards, drawn by
//! PixiJS in `atlas.js`. A card without a place waits in the list of unplaced cards beside
//! the canvas, and is placed by dragging it onto the canvas. A card on the canvas moves by
//! dragging it there, and its menu on a right click takes it off the canvas. A drag with
//! the right button, or with the left one between the cards, moves the view. The wheel
//! scales it. A right click between the cards adds a callout there, which moves and scales
//! as a card does.

use crate::core::CoreHandle;
use crate::dom;
use crate::listeners::DocumentListeners;
use leptos::prelude::*;
use peta_core::app::Event;
use peta_core::screens::atlas::{
    callout_face_svg, parent_dir_label, tag_blob_svg, tag_icon_svg, Callout, CARD_HEIGHT, CARD_WIDTH,
};
use peta_core::view::{AtlasCard, ScreenView};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

const ATLAS_DIRS_KEY: &str = "atlas-dirs";
const ATLAS_VIEWS_KEY: &str = "atlas-views";

/// The folders last seen as their atlas, as the browser stored them.
pub fn load_atlas_dirs() -> Vec<String> {
    dom::local_storage_get(ATLAS_DIRS_KEY)
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_atlas_dirs(dirs: &[String]) {
    dom::local_storage_set(ATLAS_DIRS_KEY, &serde_json::to_string(dirs).unwrap_or_default());
}

/// Where the hand tool and the wheel left the view of an atlas: how far it is moved from
/// the center of the canvas, and how much it is scaled.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
struct View {
    x: f64,
    y: f64,
    zoom: f64,
}

impl Default for View {
    fn default() -> View {
        View { x: 0.0, y: 0.0, zoom: 1.0 }
    }
}

/// The view of each atlas as the browser stored it, by the dir path of the atlas.
fn load_views() -> HashMap<String, View> {
    dom::local_storage_get(ATLAS_VIEWS_KEY)
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_view(dir: &str, view: View) {
    let mut views = load_views();
    views.insert(dir.to_string(), view);
    dom::local_storage_set(ATLAS_VIEWS_KEY, &serde_json::to_string(&views).unwrap_or_default());
}

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
        on_menu: &js_sys::Function,
        on_view: &js_sys::Function,
        pan_x: f64,
        pan_y: f64,
        zoom: f64,
    ) -> AtlasCanvas;

    #[wasm_bindgen(method, js_name = setOnGroup)]
    fn set_on_group(this: &AtlasCanvas, on_group: &js_sys::Function);

    #[wasm_bindgen(method, js_name = setOnUngroup)]
    fn set_on_ungroup(this: &AtlasCanvas, on_ungroup: &js_sys::Function);

    #[wasm_bindgen(method, js_name = setOnScale)]
    fn set_on_scale(this: &AtlasCanvas, on_scale: &js_sys::Function);

    #[wasm_bindgen(method, js_name = setOnGroupTitle)]
    fn set_on_group_title(this: &AtlasCanvas, on_group_title: &js_sys::Function);

    #[wasm_bindgen(method, js_name = setOnAddCallout)]
    fn set_on_add_callout(this: &AtlasCanvas, on_add_callout: &js_sys::Function);

    #[wasm_bindgen(method, js_name = setOnCallout)]
    fn set_on_callout(this: &AtlasCanvas, on_callout: &js_sys::Function);

    #[wasm_bindgen(method, js_name = setOnMoveCallout)]
    fn set_on_move_callout(this: &AtlasCanvas, on_move_callout: &js_sys::Function);

    #[wasm_bindgen(method, js_name = setOnScaleCallout)]
    fn set_on_scale_callout(this: &AtlasCanvas, on_scale_callout: &js_sys::Function);

    #[wasm_bindgen(method, js_name = setCards)]
    fn set_cards(this: &AtlasCanvas, json: String);

    #[wasm_bindgen(method, js_name = setCallouts)]
    fn set_callouts(this: &AtlasCanvas, json: String);

    #[wasm_bindgen(method, js_name = toWorldX)]
    fn to_world_x(this: &AtlasCanvas, client_x: f64) -> Option<f64>;

    #[wasm_bindgen(method, js_name = toWorldY)]
    fn to_world_y(this: &AtlasCanvas, client_y: f64) -> Option<f64>;

    #[wasm_bindgen(method)]
    fn destroy(this: &AtlasCanvas);
}

/// The size of the icon of a card, in pixels of the canvas.
const ICON_SIZE: i32 = 48;
/// The size the icon is drawn at before it is scaled down to the card. The hatching of
/// picturesque-marks has a fixed size made for an icon of about 100px; drawn small, a shape
/// would hold only a few of its dots or lines.
const ICON_DRAW_SIZE: i32 = 100;

/// The placed cards as `atlas.js` reads them. The icons are drawn here rather than in the
/// view, so they are drawn only when the cards change.
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
                    "scale": b.scale(),
                    "group": c.group,
                    "groupColor": c.group_color,
                    "groupTitle": c.group_title.as_ref().map(|t| &t.text),
                    "groupTitleSide": c.group_title.as_ref().map(|t| &t.side),
                    "icon": (!c.icon_tags.is_empty())
                        .then(|| tag_icon_svg(&c.icon_tags, ICON_DRAW_SIZE)),
                    // The blob is drawn from the main tag, which comes first
                    "blob": c.icon_tags.first().map(|tag| tag_blob_svg(tag, ICON_DRAW_SIZE)),
                    "iconSize": ICON_SIZE,
                    "iconDrawSize": ICON_DRAW_SIZE,
                })
            })
        })
        .collect();
    serde_json::Value::Array(placed).to_string()
}

/// The size the character of a callout is drawn at, as the icons are
const CALLOUT_DRAW_SIZE: i32 = 100;

/// The callouts as `atlas.js` reads them, each with its character drawn from its text.
fn callouts_json(callouts: &[Callout]) -> String {
    let all: Vec<_> = callouts
        .iter()
        .map(|c| {
            serde_json::json!({
                "key": c.key,
                "x": c.x,
                "y": c.y,
                "scale": c.scale,
                "side": c.side,
                "text": c.text,
                "character": callout_face_svg(&c.text, CALLOUT_DRAW_SIZE),
                "drawSize": CALLOUT_DRAW_SIZE,
            })
        })
        .collect();
    serde_json::Value::Array(all).to_string()
}

#[component]
pub fn Atlas() -> impl IntoView {
    let core: CoreHandle = expect_context();
    let cards = core.select(|v| v.atlas_cards.clone());
    let callouts = core.select(|v| v.atlas_callouts.clone());
    let dir_path = core.select(|v| match &v.screen {
        ScreenView::Page(Some(p)) => p.page.dir_path.clone(),
        _ => String::new(),
    });
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

    // The menu of a right-clicked card: its page and where it opens on the screen
    let menu = RwSignal::new(None::<(String, f64, f64)>);
    // The click outside and the Escape that close the open menu
    let menu_listeners = DocumentListeners::new();
    let close_menu = move || {
        menu.set(None);
        menu_listeners.clear_later();
    };
    let on_menu = StoredValue::new_local(Closure::<dyn Fn(String, f64, f64)>::new(
        move |page_id: String, x: f64, y: f64| {
            menu_listeners.clear_later();
            menu.set(Some((page_id, x, y)));
            // Listening starts after this click, so the click itself does not close the menu
            dom::after_render(move || {
                menu_listeners.on_cancelable("mousedown", move |ev: &web_sys::MouseEvent| {
                    let inside_menu = ev
                        .target()
                        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                        .is_some_and(|t| t.closest(".atlas-menu").ok().flatten().is_some());
                    if !inside_menu {
                        close_menu();
                    }
                });
                menu_listeners.on("keydown", move |ev: &web_sys::KeyboardEvent| {
                    if ev.key() == "Escape" {
                        ev.prevent_default();
                        close_menu();
                    }
                });
            });
        },
    ));
    // The directory whose view the canvas shows, fixed when the canvas is made: the view
    // is kept for it even if the page changes before the canvas is gone
    let view_dir = StoredValue::new(String::new());
    let on_view = StoredValue::new_local(Closure::<dyn Fn(f64, f64, f64)>::new(
        move |x: f64, y: f64, zoom: f64| {
            save_view(&view_dir.get_value(), View { x, y, zoom });
        },
    ));
    // Cards dropped on a card with Ctrl held: the ids of the dropped cards come as JSON
    let on_group = StoredValue::new_local(Closure::<dyn Fn(String, String)>::new(
        move |ids_json: String, target_id: String| {
            if let Ok(page_ids) = serde_json::from_str::<Vec<String>>(&ids_json) {
                core.send(Event::GroupAtlasCards { page_ids, target_id });
            }
        },
    ));
    // Cards dragged out of the outline of their group with Ctrl held, as JSON
    let on_ungroup = StoredValue::new_local(Closure::<dyn Fn(String)>::new(move |ids_json: String| {
        if let Ok(page_ids) = serde_json::from_str::<Vec<String>>(&ids_json) {
            core.send(Event::UngroupAtlasCards { page_ids });
        }
    }));
    // A card scaled with Ctrl and the wheel
    let on_scale = StoredValue::new_local(Closure::<dyn Fn(String, f64)>::new(
        move |page_id: String, scale: f64| {
            core.send(Event::ScaleAtlasCard { page_id, scale });
        },
    ));
    // The title of a group given in its dialog, along a side of its outline
    let on_group_title = StoredValue::new_local(Closure::<dyn Fn(String, String, String)>::new(
        move |group_key: String, side: String, title: String| {
            core.send(Event::SetAtlasGroupTitle { group_key, side, title });
        },
    ));
    // A callout added in its dialog, its character centered at a point of the canvas
    let on_add_callout = StoredValue::new_local(Closure::<dyn Fn(f64, f64, String, String)>::new(
        move |x: f64, y: f64, side: String, text: String| {
            core.send(Event::AddAtlasCallout { x, y, side, text });
        },
    ));
    // A callout changed in its dialog
    let on_callout = StoredValue::new_local(Closure::<dyn Fn(String, String, String)>::new(
        move |key: String, side: String, text: String| {
            core.send(Event::SetAtlasCallout { key, side, text });
        },
    ));
    // A callout dragged to a new place
    let on_move_callout = StoredValue::new_local(Closure::<dyn Fn(String, f64, f64)>::new(
        move |key: String, x: f64, y: f64| {
            core.send(Event::MoveAtlasCallout { key, x, y });
        },
    ));
    // A callout scaled with Ctrl and the wheel
    let on_scale_callout = StoredValue::new_local(Closure::<dyn Fn(String, f64)>::new(
        move |key: String, scale: f64| {
            core.send(Event::ScaleAtlasCallout { key, scale });
        },
    ));
    let remove_from_atlas = move |_| {
        if let Some((page_id, _, _)) = menu.get_untracked() {
            core.send(Event::RemoveAtlasCard { page_id });
        }
        close_menu();
    };

    // The canvas, made by whichever of the cards and the callouts comes first
    let with_canvas = move |f: &dyn Fn(&AtlasCanvas)| {
        let Some(el) = host.get() else {
            return;
        };
        canvas.update_value(|c| {
            f(c.get_or_insert_with(|| {
                let dir = dir_path.get_untracked();
                let start = load_views().get(&dir).copied().unwrap_or_default();
                view_dir.set_value(dir);
                let made = on_open.with_value(|open| {
                    on_place.with_value(|place| {
                        on_menu.with_value(|menu| {
                            on_view.with_value(|keep| {
                                AtlasCanvas::new(
                                    &el,
                                    open.as_ref().unchecked_ref(),
                                    place.as_ref().unchecked_ref(),
                                    menu.as_ref().unchecked_ref(),
                                    keep.as_ref().unchecked_ref(),
                                    start.x,
                                    start.y,
                                    start.zoom,
                                )
                            })
                        })
                    })
                });
                on_group.with_value(|group| made.set_on_group(group.as_ref().unchecked_ref()));
                on_ungroup.with_value(|ungroup| made.set_on_ungroup(ungroup.as_ref().unchecked_ref()));
                on_scale.with_value(|scale| made.set_on_scale(scale.as_ref().unchecked_ref()));
                on_group_title.with_value(|title| made.set_on_group_title(title.as_ref().unchecked_ref()));
                on_add_callout.with_value(|add| made.set_on_add_callout(add.as_ref().unchecked_ref()));
                on_callout.with_value(|change| made.set_on_callout(change.as_ref().unchecked_ref()));
                on_move_callout.with_value(|mv| made.set_on_move_callout(mv.as_ref().unchecked_ref()));
                on_scale_callout.with_value(|sc| made.set_on_scale_callout(sc.as_ref().unchecked_ref()));
                made
            }))
        });
    };
    // Apart, so a change of a callout does not draw every card again
    Effect::new(move |_| {
        let json = cards.with(|cs| placed_json(cs));
        with_canvas(&|c| c.set_cards(json.clone()));
    });
    Effect::new(move |_| {
        let json = callouts.with(|cs| callouts_json(cs));
        with_canvas(&|c| c.set_callouts(json.clone()));
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
        // Only the canvas knows how far the hand tool has moved the view
        let point = canvas.with_value(|c| {
            let c = c.as_ref()?;
            Some((c.to_world_x(ev.client_x() as f64)?, c.to_world_y(ev.client_y() as f64)?))
        });
        let Some((x, y)) = point else {
            return;
        };
        // The grabbed spot of the card stays under the pointer, as the dragged image shows it.
        // The card is placed by its center, which lies this far from the grabbed spot.
        core.send(Event::PlaceAtlasCard {
            page_id: grab.page_id,
            x: x + (0.5 - grab.x) * CARD_WIDTH,
            y: y + (0.5 - grab.y) * CARD_HEIGHT,
        });
    };

    view! {
        <div class="atlas">
            <div class="atlas-unplaced">
                <For
                    each=move || unplaced.get()
                <div class="atlas-unplaced-title">"Unplaced"</div>
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
            // The browser's own menu would cover the menu of a card
            <div class="atlas-canvas" node_ref=host
                on:dragover=move |ev: web_sys::DragEvent| ev.prevent_default()
                on:drop=on_drop
                on:contextmenu=move |ev: web_sys::MouseEvent| ev.prevent_default()>
                // HTML over the canvas, not drawn by PixiJS: it keeps its corner whatever the
                // canvas shows. It names the directory it goes to. The root has no directory above.
                <Show when=move || dir_path.with(|d| !d.is_empty())>
                    <button class="atlas-up-btn" title="Up" on:click=move |_| core.send(Event::AtlasUp)>
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <polyline points="14 9 9 4 4 9"/>
                            <path d="M20 20h-7a4 4 0 0 1-4-4V4"/>
                        </svg>
                        <span>{move || dir_path.with(|d| parent_dir_label(d))}</span>
                    </button>
                </Show>
            </div>
            {move || menu.get().map(|(_, x, y)| view! {
                <div class="atlas-menu" style=format!("left: {x}px; top: {y}px;")>
                    <button class="atlas-menu-item" on:click=remove_from_atlas>"Remove from Atlas"</button>
                </div>
            })}
        </div>
    }
}
