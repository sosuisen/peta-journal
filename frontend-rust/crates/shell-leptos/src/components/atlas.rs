//! The canvas of an atlas page: the pages and folders of its directory as cards, drawn by
//! PixiJS in `atlas.js`. A card without a place waits in the list of unplaced cards beside
//! the canvas, and is placed by dragging it onto the canvas. With Ctrl held, it also joins
//! the group of the card or the group area it is dropped on. A card on the canvas moves by
//! dragging it there, and its menu on a right click takes it off the canvas. A drag with
//! the right button, or with the left one between the cards, moves the view. The wheel
//! scales it. A right click between the cards adds a callout or a link card there. Both move
//! and scale as a card does. A link card is a wiki link in the atlas page to any page.

use crate::core::CoreHandle;
use crate::dom;
use crate::listeners::DocumentListeners;
use leptos::prelude::*;
use peta_atlas::scene::{self, GroupRef};
use peta_core::app::Event;
use peta_core::screens::atlas::{
    parent_dir_label, Callout, GroupTitle, LinkCard, CARD_HEIGHT, CARD_WIDTH,
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
    dom::local_storage_set(
        ATLAS_DIRS_KEY,
        &serde_json::to_string(dirs).unwrap_or_default(),
    );
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
        View {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        }
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
    dom::local_storage_set(
        ATLAS_VIEWS_KEY,
        &serde_json::to_string(&views).unwrap_or_default(),
    );
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
        options: &JsValue,
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

    #[wasm_bindgen(method, js_name = setOnLink)]
    fn set_on_link(
        this: &AtlasCanvas,
        on_add: &js_sys::Function,
        on_set: &js_sys::Function,
        on_remove: &js_sys::Function,
        on_query: &js_sys::Function,
    );

    #[wasm_bindgen(method, js_name = setLinkSuggestions)]
    fn set_link_suggestions(this: &AtlasCanvas, json: String);

    #[wasm_bindgen(method, js_name = setCards)]
    fn set_cards(this: &AtlasCanvas, json: String);

    #[wasm_bindgen(method, js_name = setCallouts)]
    fn set_callouts(this: &AtlasCanvas, json: String);

    #[wasm_bindgen(method, js_name = toWorldX)]
    fn to_world_x(this: &AtlasCanvas, client_x: f64) -> Option<f64>;

    #[wasm_bindgen(method, js_name = toWorldY)]
    fn to_world_y(this: &AtlasCanvas, client_y: f64) -> Option<f64>;

    #[wasm_bindgen(method, js_name = dragOverFromList)]
    fn drag_over_from_list(this: &AtlasCanvas, client_x: f64, client_y: f64, ctrl: bool);

    #[wasm_bindgen(method, js_name = endDragFromList)]
    fn end_drag_from_list(this: &AtlasCanvas);

    #[wasm_bindgen(method, js_name = groupTargetFromList)]
    fn group_target_from_list(
        this: &AtlasCanvas,
        client_x: f64,
        client_y: f64,
        ctrl: bool,
    ) -> Option<String>;

    #[wasm_bindgen(method)]
    fn destroy(this: &AtlasCanvas);
}

/// The options of the canvas: the labels in the font that a static site draws an atlas in
/// too, so a label takes the same width in both. index.html copies the font to the root.
const CANVAS_OPTIONS: &str =
    r#"{ "labelFont": { "family": "PetaAtlas Label", "url": "/NotoSansJP-Regular.ttf" } }"#;

fn canvas_options() -> JsValue {
    js_sys::JSON::parse(CANVAS_OPTIONS).expect("the options are JSON")
}

/// The group of a card as the scene takes it.
fn group_ref(group: Option<&String>, title: Option<&GroupTitle>) -> Option<GroupRef> {
    group.map(|id| GroupRef {
        id: id.clone(),
        title: title.map(|t| (t.text.clone(), t.side.clone())),
    })
}

/// The placed cards as `atlas.js` reads them, the link cards among them. The icons are drawn
/// here rather than in the view, so they are drawn only when the cards change.
fn placed_json(cards: &[AtlasCard], links: &[LinkCard]) -> String {
    let mut placed: Vec<scene::Card> = cards
        .iter()
        .filter_map(|c| {
            let page = scene::Page {
                id: &c.page_id,
                label: &c.label,
                path: &c.path,
                is_dir: c.is_dir,
                icon_tags: &c.icon_tags,
                published: c.published,
            };
            let group = group_ref(c.group.as_ref(), c.group_title.as_ref());
            c.bounds
                .map(|b| scene::Card::page(&page, b, group.as_ref()))
        })
        .collect();
    placed.extend(links.iter().map(|l| {
        let group = group_ref(l.group.as_ref(), l.group_title.as_ref());
        scene::Card::link(&l.item_id, &l.target, l.bounds, group.as_ref())
    }));
    scene::cards_json(&placed)
}

/// The callouts as `atlas.js` reads them, each with its character drawn from its text.
fn callouts_json(callouts: &[Callout]) -> String {
    let all: Vec<scene::Callout> = callouts
        .iter()
        .map(|c| scene::Callout::new(&c.key, c.x, c.y, c.scale, &c.side, &c.text))
        .collect();
    scene::callouts_json(&all)
}

#[component]
pub fn Atlas() -> impl IntoView {
    let core: CoreHandle = expect_context();
    let cards = core.select(|v| v.atlas_cards.clone());
    let callouts = core.select(|v| v.atlas_callouts.clone());
    let links = core.select(|v| v.atlas_links.clone());
    let link_suggestions = core.select(|v| v.atlas_link_suggestions.clone());
    let dir_path = core.select(|v| match &v.screen {
        ScreenView::Page(Some(p)) => p.page.dir_path.clone(),
        _ => String::new(),
    });
    let unplaced = Memo::new(move |_| {
        cards.with(|cs| {
            cs.iter()
                .filter(|c| c.bounds.is_none())
                .cloned()
                .collect::<Vec<_>>()
        })
    });
    let host = NodeRef::<leptos::html::Div>::new();
    let canvas = StoredValue::new_local(None::<AtlasCanvas>);
    // A click on the label of a card on the canvas; kept as long as the canvas lives
    let on_open = StoredValue::new_local(Closure::<dyn Fn(String, bool, bool)>::new(
        move |path: String, new_tab: bool, new_window: bool| {
            core.send(Event::GoToPage {
                path,
                new_tab,
                new_window,
            });
        },
    ));
    // A card dragged to a new place on the canvas
    let on_place = StoredValue::new_local(Closure::<dyn Fn(String, f64, f64)>::new(
        move |card_id: String, x: f64, y: f64| {
            core.send(Event::PlaceAtlasCard { card_id, x, y });
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
            menu_listeners.close_menu_on_outside_or_escape(".atlas-menu", close_menu);
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
            if let Ok(card_ids) = serde_json::from_str::<Vec<String>>(&ids_json) {
                core.send(Event::GroupAtlasCards {
                    card_ids,
                    target_id,
                });
            }
        },
    ));
    // Cards dragged out of the outline of their group with Ctrl held, as JSON
    let on_ungroup =
        StoredValue::new_local(Closure::<dyn Fn(String)>::new(move |ids_json: String| {
            if let Ok(card_ids) = serde_json::from_str::<Vec<String>>(&ids_json) {
                core.send(Event::UngroupAtlasCards { card_ids });
            }
        }));
    // A card scaled with Ctrl and the wheel
    let on_scale = StoredValue::new_local(Closure::<dyn Fn(String, f64)>::new(
        move |card_id: String, scale: f64| {
            core.send(Event::ScaleAtlasCard { card_id, scale });
        },
    ));
    // The title of a group given in its dialog, along a side of its outline
    let on_group_title = StoredValue::new_local(Closure::<dyn Fn(String, String, String)>::new(
        move |group_id: String, side: String, title: String| {
            core.send(Event::SetAtlasGroupTitle {
                group_id,
                side,
                title,
            });
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
    // A link card added in its dialog, its center at a point of the canvas
    let on_add_link = StoredValue::new_local(Closure::<dyn Fn(f64, f64, String)>::new(
        move |x: f64, y: f64, target: String| {
            core.send(Event::AddAtlasLink { x, y, target });
        },
    ));
    // A link card linked to another page in its dialog
    let on_set_link = StoredValue::new_local(Closure::<dyn Fn(String, String)>::new(
        move |item_id: String, target: String| {
            core.send(Event::SetAtlasLink { item_id, target });
        },
    ));
    let on_remove_link =
        StoredValue::new_local(Closure::<dyn Fn(String)>::new(move |item_id: String| {
            core.send(Event::RemoveAtlasLink { item_id });
        }));
    // The text typed in the dialog of a link
    let on_link_query =
        StoredValue::new_local(Closure::<dyn Fn(String)>::new(move |query: String| {
            core.send(Event::SearchAtlasLinks { query });
        }));
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
                                    &canvas_options(),
                                )
                            })
                        })
                    })
                });
                on_group.with_value(|group| made.set_on_group(group.as_ref().unchecked_ref()));
                on_ungroup
                    .with_value(|ungroup| made.set_on_ungroup(ungroup.as_ref().unchecked_ref()));
                on_scale.with_value(|scale| made.set_on_scale(scale.as_ref().unchecked_ref()));
                on_group_title
                    .with_value(|title| made.set_on_group_title(title.as_ref().unchecked_ref()));
                on_add_callout
                    .with_value(|add| made.set_on_add_callout(add.as_ref().unchecked_ref()));
                on_callout
                    .with_value(|change| made.set_on_callout(change.as_ref().unchecked_ref()));
                on_move_callout
                    .with_value(|mv| made.set_on_move_callout(mv.as_ref().unchecked_ref()));
                on_scale_callout
                    .with_value(|sc| made.set_on_scale_callout(sc.as_ref().unchecked_ref()));
                on_add_link.with_value(|add| {
                    on_set_link.with_value(|set| {
                        on_remove_link.with_value(|rm| {
                            on_link_query.with_value(|query| {
                                made.set_on_link(
                                    add.as_ref().unchecked_ref(),
                                    set.as_ref().unchecked_ref(),
                                    rm.as_ref().unchecked_ref(),
                                    query.as_ref().unchecked_ref(),
                                )
                            })
                        })
                    })
                });
                made
            }))
        });
    };
    // Apart, so a change of a callout does not draw every card again
    Effect::new(move |_| {
        let json = cards.with(|cs| links.with(|ls| placed_json(cs, ls)));
        with_canvas(&|c| c.set_cards(json.clone()));
    });
    Effect::new(move |_| {
        let json = callouts.with(|cs| callouts_json(cs));
        with_canvas(&|c| c.set_callouts(json.clone()));
    });
    Effect::new(move |_| {
        let json = link_suggestions.with(|ts| serde_json::to_string(ts).unwrap_or_default());
        with_canvas(&|c| c.set_link_suggestions(json.clone()));
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
            Some((
                c.to_world_x(ev.client_x() as f64)?,
                c.to_world_y(ev.client_y() as f64)?,
            ))
        });
        let Some((x, y)) = point else {
            return;
        };
        // With Ctrl, a card let go on a card or in the area of a group joins that group
        let ctrl = ev.ctrl_key() || ev.meta_key();
        let target_id = canvas.with_value(|c| {
            let c = c.as_ref()?;
            c.end_drag_from_list();
            c.group_target_from_list(ev.client_x() as f64, ev.client_y() as f64, ctrl)
        });
        // The grabbed spot of the card stays under the pointer, as the dragged image shows it.
        // The card is placed by its center, which lies this far from the grabbed spot.
        core.send(Event::PlaceAtlasCard {
            card_id: grab.page_id.clone(),
            x: x + (0.5 - grab.x) * CARD_WIDTH,
            y: y + (0.5 - grab.y) * CARD_HEIGHT,
        });
        if let Some(target_id) = target_id {
            core.send(Event::GroupAtlasCards {
                card_ids: vec![grab.page_id],
                target_id,
            });
        }
    };
    let on_drag_over = move |ev: web_sys::DragEvent| {
        ev.prevent_default();
        let Some(dt) = ev.data_transfer() else {
            return;
        };
        // Ctrl would make the drop a copy, which the card does not allow
        dt.set_drop_effect("move");
        // Text dragged from elsewhere joins no group
        if !dt.types().includes(&CARD_MIME.into(), 0) {
            return;
        }
        let ctrl = ev.ctrl_key() || ev.meta_key();
        canvas.with_value(|c| {
            if let Some(c) = c.as_ref() {
                c.drag_over_from_list(ev.client_x() as f64, ev.client_y() as f64, ctrl);
            }
        });
    };
    // Also when the pointer only passes onto a button over the canvas; the next dragover
    // shows the group again
    let on_drag_leave = move |_| {
        canvas.with_value(|c| {
            if let Some(c) = c.as_ref() {
                c.end_drag_from_list();
            }
        });
    };

    view! {
        <div class="atlas">
            <div class="atlas-unplaced">
                <div class="atlas-unplaced-title">"Unplaced"</div>
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
                                    // The card, not the label inside it that may be under the pointer
                                    let Some(el) = dom::target_closest(&ev, ".atlas-card") else {
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
                on:dragover=on_drag_over
                on:dragleave=on_drag_leave
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
