//! The outline, ported from `outline.ts`: a windowed list of rows (only the rows near the
//! viewport are in the DOM, the rest is padding measured from the rows' real heights), the
//! scroll reports the journal needs, scrolling to an item, and the document listeners of
//! selection mode.

use crate::components::outline_item::OutlineItem;
use crate::components::wiki_suggestions::WikiSuggestions;
use crate::core::CoreHandle;
use crate::dom;
use gloo_events::EventListener;
use leptos::prelude::*;
use peta_core::app::SelectionKey;
use peta_core::view::Row;
use peta_core::Event;
use send_wrapper::SendWrapper;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

type Heights = SendWrapper<Rc<RefCell<HashMap<String, f64>>>>;
use wasm_bindgen::JsCast;

/// Pixels rendered above and below the viewport, like the CDK's min/max buffer.
const MIN_BUFFER_PX: f64 = 500.0;
const MAX_BUFFER_PX: f64 = 1000.0;
/// The height of a row that was never measured.
const ESTIMATED_ROW_PX: f64 = 28.0;

#[derive(Clone, Copy, PartialEq, Debug, Default)]
struct Window {
    start: usize,
    end: usize,
    pad_top: f64,
    pad_bottom: f64,
}

/// Rows by id with one signal each, so a change to one item re-renders one row.
#[derive(Clone)]
struct RowStore {
    ids: RwSignal<Vec<String>>,
    rows: SendWrapper<Rc<RefCell<HashMap<String, RwSignal<Row>>>>>,
}

impl RowStore {
    fn new(vm: RwSignal<peta_core::ViewModel>) -> RowStore {
        let store = RowStore { ids: RwSignal::new(Vec::new()), rows: SendWrapper::new(Rc::new(RefCell::new(HashMap::new()))) };
        let s = store.clone();
        Effect::new(move |_| {
            vm.with(|v| {
                let mut map = s.rows.borrow_mut();
                let mut new_ids = Vec::with_capacity(v.outline.rows.len());
                for row in &v.outline.rows {
                    new_ids.push(row.item.id.clone());
                    match map.get(&row.item.id) {
                        Some(sig) => {
                            if sig.with_untracked(|r| r != row) {
                                sig.set(row.clone());
                            }
                        }
                        None => {
                            map.insert(row.item.id.clone(), RwSignal::new(row.clone()));
                        }
                    }
                }
                let keep: std::collections::HashSet<&String> = new_ids.iter().collect();
                map.retain(|id, _| keep.contains(id));
                drop(map);
                if s.ids.with_untracked(|cur| *cur != new_ids) {
                    s.ids.set(new_ids);
                }
            });
        });
        store
    }

    fn row(&self, id: &str) -> Option<RwSignal<Row>> {
        self.rows.borrow().get(id).copied()
    }
}

#[component]
pub fn Outline() -> impl IntoView {
    let core: CoreHandle = expect_context();
    let vm = core.view();
    let store = RowStore::new(vm);
    let viewport = NodeRef::<leptos::html::Custom<&'static str>>::new();
    let wrapper = NodeRef::<leptos::html::Div>::new();
    let heights: Heights = SendWrapper::new(Rc::new(RefCell::new(HashMap::new())));
    let window = RwSignal::new(Window::default());
    let measure_scheduled = SendWrapper::new(Rc::new(RefCell::new(false)));

    // Recomputes the rendered range from the scroll position and the known heights.
    let recompute = {
        let heights = heights.clone();
        let ids = store.ids;
        SendWrapper::new(Rc::new(move || {
            let Some(vp) = viewport.get_untracked() else { return };
            let scroll_top = vp.scroll_top() as f64;
            let vp_height = vp.client_height() as f64;
            let heights = heights.borrow();
            let w = ids.with_untracked(|ids| {
                let height_of = |id: &String| heights.get(id).copied().unwrap_or(ESTIMATED_ROW_PX);
                let top_edge = scroll_top - MIN_BUFFER_PX;
                let bottom_edge = scroll_top + vp_height + MAX_BUFFER_PX;
                let mut y = 0.0;
                let mut start = ids.len();
                let mut end = ids.len();
                let mut pad_top = 0.0;
                let mut pad_bottom = 0.0;
                for (i, id) in ids.iter().enumerate() {
                    let h = height_of(id);
                    if start == ids.len() && y + h > top_edge {
                        start = i;
                        pad_top = y;
                    }
                    if start != ids.len() && end == ids.len() && y >= bottom_edge {
                        end = i;
                    }
                    if end != ids.len() {
                        pad_bottom += h;
                    }
                    y += h;
                }
                if start == ids.len() {
                    start = ids.len().saturating_sub(1);
                    pad_top = (y - height_of(ids.last().unwrap_or(&String::new()))).max(0.0);
                    end = ids.len();
                    pad_bottom = 0.0;
                }
                Window { start, end, pad_top, pad_bottom }
            });
            if window.get_untracked() != w {
                window.set(w);
            }
        }))
    };

    // Measures the rendered rows after each render; a change re-runs the window computation.
    let schedule_measure = {
        let heights = heights.clone();
        let recompute = recompute.clone();
        let scheduled = measure_scheduled.clone();
        SendWrapper::new(Rc::new(move || {
            if *scheduled.borrow() {
                return;
            }
            *scheduled.borrow_mut() = true;
            let heights = heights.clone();
            let recompute = recompute.clone();
            let scheduled = scheduled.clone();
            request_animation_frame(move || {
                *scheduled.borrow_mut() = false;
                let Some(wrap) = wrapper.get_untracked() else { return };
                let children = wrap.children();
                let mut changed = false;
                let mut map = heights.borrow_mut();
                for i in 0..children.length() {
                    let Some(el) = children.item(i).and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) else { continue };
                    let Some(id) = el.get_attribute("data-row-id") else { continue };
                    let h = el.offset_height() as f64;
                    if h > 0.0 && map.get(&id).map(|old| (old - h).abs() > 0.5).unwrap_or(true) {
                        map.insert(id, h);
                        changed = true;
                    }
                }
                drop(map);
                if changed {
                    recompute();
                }
            });
        }))
    };

    // The window follows the rows and the scroll position
    {
        let recompute = recompute.clone();
        let sm = schedule_measure.clone();
        let ids = store.ids;
        Effect::new(move |_| {
            ids.track();
            recompute();
            sm();
        });
        let schedule_measure = schedule_measure.clone();
        Effect::new(move |_| {
            window.track();
            schedule_measure();
        });
    }

    // Scroll: the top item and the buffer the journal keeps ahead
    let on_scroll = {
                let recompute = recompute.clone();
        let heights = heights.clone();
        let ids = store.ids;
        move |_| {
            recompute();
            let Some(vp) = viewport.get_untracked() else { return };
            let scroll_top = vp.scroll_top() as f64;
            let w = window.get_untracked();
            // The first rendered row whose bottom is below the viewport top
            let top_visible = {
                let heights = heights.borrow();
                let mut y = w.pad_top;
                let mut found = w.start;
                ids.with_untracked(|ids| {
                    for (i, id) in ids.iter().enumerate().skip(w.start).take(w.end.saturating_sub(w.start)) {
                        y += heights.get(id).copied().unwrap_or(ESTIMATED_ROW_PX);
                        if y > scroll_top {
                            found = i;
                            break;
                        }
                    }
                });
                found
            };
            core.send(Event::OutlineScrolled {
                scrolled_from_top: scroll_top > 0.0,
                top_visible_index: top_visible,
                rendered_start: w.start,
                rendered_end: w.end,
            });
        }
    };

    // Scrolling to an item the sidebar or a search result asked for
    {
        let heights = heights.clone();
        let ids = store.ids;
        let scroller: crate::core::ScrollToItem = Box::new(move |item_id: String, smooth: bool| {
            scroll_to_item(viewport, &heights, ids, window, item_id, smooth, 0);
        });
        core.set_scroller(Some(scroller));
                on_cleanup(move || core.set_scroller(None));
    }

    // Selection mode: the keys and clicks of the whole document act on the selection
    {
                let listeners: SendWrapper<Rc<RefCell<Vec<EventListener>>>> = SendWrapper::new(Rc::new(RefCell::new(Vec::new())));
        let l2 = listeners.clone();
        Effect::new(move |_| {
            let active = vm.with(|v| v.outline.selection_mode);
            let mut ls = l2.borrow_mut();
            ls.clear();
            if !active {
                return;
            }
            let document = dom::document();
            let c = core;
            ls.push(EventListener::new_with_options(
                &document,
                "keydown",
                gloo_events::EventListenerOptions::enable_prevent_default(),
                move |ev| {
                    let ev: &web_sys::KeyboardEvent = ev.unchecked_ref();
                    if let Some(key) = selection_key(ev) {
                        ev.prevent_default();
                        c.send(Event::SelectionKey(key));
                    }
                },
            ));
            let c = core;
            ls.push(EventListener::new(&document, "click", move |_| c.send(Event::SelectionClickOutside)));
            let c = core;
            ls.push(EventListener::new(&document, "mouseup", move |_| c.send(Event::SelectionMouseUp)));
            for name in ["copy", "cut"] {
                let c = core;
                ls.push(EventListener::new_with_options(
                    &document,
                    name,
                    gloo_events::EventListenerOptions::enable_prevent_default(),
                    move |ev| {
                        ev.prevent_default();
                        c.send(Event::SelectionKey(if name == "copy" { SelectionKey::Copy } else { SelectionKey::Cut }));
                    },
                ));
            }
        });
        on_cleanup(move || listeners.borrow_mut().clear());
    }

    let rendered_ids = {
        let ids = store.ids;
        move || {
            let w = window.get();
            ids.with(|ids| ids.iter().skip(w.start).take(w.end.saturating_sub(w.start)).cloned().collect::<Vec<_>>())
        }
    };

    view! {
        <app-outline>
            <cdk-virtual-scroll-viewport node_ref=viewport class="outline-viewport" on:scroll=on_scroll>
                <div
                    node_ref=wrapper
                    class="cdk-virtual-scroll-content-wrapper"
                    style:padding-top=move || format!("{}px", window.get().pad_top)
                    style:padding-bottom=move || format!("{}px", window.get().pad_bottom)
                >
                    <For
                        each=rendered_ids
                        key=|id| id.clone()
                        children={
                            let store = store.clone();
                                                        move |id| {
                                let Some(row) = store.row(&id) else { return ().into_any() };
                                                                let id2 = id.clone();
                                let id3 = id.clone();
                                view! {
                                    <div
                                        class="outline-row"
                                        attr:data-row-id=id.clone()
                                        on:mouseenter=move |_| core.send(Event::ItemMouseEnter { item_id: id2.clone() })
                                        on:wheel={
                                                                                        move |ev: web_sys::WheelEvent| {
                                                if !ev.shift_key() { return; }
                                                ev.prevent_default();
                                                core.send(Event::ZoomStep { item_id: id3.clone(), up: ev.delta_y() < 0.0 });
                                            }
                                        }
                                    >
                                        {move || row.with(|r| r.date_header.clone()).map(|h| view! { <div class="date-header">{h}</div> })}
                                        <OutlineItem row=row />
                                    </div>
                                }
                                .into_any()
                            }
                        }
                    />
                </div>
            </cdk-virtual-scroll-viewport>
            <WikiSuggestions />
        </app-outline>
    }
}

fn selection_key(ev: &web_sys::KeyboardEvent) -> Option<SelectionKey> {
    let key = ev.key();
    if key == "Escape" {
        return Some(SelectionKey::Escape);
    }
    if (key == "z" || key == "Z") && ev.ctrl_key() && !ev.alt_key() {
        return Some(if ev.shift_key() { SelectionKey::Redo } else { SelectionKey::Undo });
    }
    if key == "Delete" || key == "Backspace" {
        return Some(SelectionKey::Delete);
    }
    if ev.shift_key() && ev.alt_key() {
        return match key.as_str() {
            "ArrowUp" => Some(SelectionKey::MoveUp),
            "ArrowDown" => Some(SelectionKey::MoveDown),
            "ArrowLeft" => Some(SelectionKey::Outdent),
            "ArrowRight" => Some(SelectionKey::Indent),
            _ => None,
        };
    }
    if ev.shift_key() && !ev.alt_key() && !ev.ctrl_key() {
        return match key.as_str() {
            "ArrowUp" => Some(SelectionKey::ExtendUp),
            "ArrowDown" => Some(SelectionKey::ExtendDown),
            _ => None,
        };
    }
    if key == "Tab" {
        return Some(if ev.shift_key() { SelectionKey::Outdent } else { SelectionKey::Indent });
    }
    if ev.ctrl_key() && key == "c" {
        return Some(SelectionKey::Copy);
    }
    if ev.ctrl_key() && key == "x" {
        return Some(SelectionKey::Cut);
    }
    None
}

/// Scrolls the item to a tenth from the top of the viewport. An item that is not rendered
/// yet is approached by its estimated offset and then step by step until it appears.
#[allow(clippy::too_many_arguments)]
fn scroll_to_item(
    viewport: NodeRef<leptos::html::Custom<&'static str>>,
    heights: &Heights,
    ids: RwSignal<Vec<String>>,
    window: RwSignal<Window>,
    item_id: String,
    smooth: bool,
    attempt: u32,
) {
    let Some(vp) = viewport.get_untracked() else { return };
    if let Some(el) = dom::element_by_id(&item_id) {
        let el_rect = el.get_bounding_client_rect();
        let vp_rect = vp.get_bounding_client_rect();
        let target_y = vp_rect.top() + vp_rect.height() * 0.1;
        let options = web_sys::ScrollToOptions::new();
        options.set_top(el_rect.top() - target_y);
        options.set_behavior(if smooth || attempt > 0 { web_sys::ScrollBehavior::Smooth } else { web_sys::ScrollBehavior::Auto });
        vp.scroll_by_with_scroll_to_options(&options);
        return;
    }
    if attempt > 50 {
        return;
    }
    let Some(index) = ids.with_untracked(|ids| ids.iter().position(|id| *id == item_id)) else { return };
    let vp_height = vp.client_height() as f64;
    if attempt == 0 {
        // Estimate the position and jump
        let estimated = {
            let heights = heights.borrow();
            ids.with_untracked(|ids| {
                ids.iter().take(index).map(|id| heights.get(id).copied().unwrap_or(ESTIMATED_ROW_PX)).sum::<f64>()
            })
        };
        vp.set_scroll_top((estimated - vp_height * 0.1).max(0.0) as i32);
    } else {
        let w = window.get_untracked();
        if index >= w.end {
            vp.set_scroll_top(vp.scroll_top() + (vp_height * 0.8) as i32);
        } else if index < w.start {
            vp.set_scroll_top((vp.scroll_top() - (vp_height * 0.8) as i32).max(0));
        }
    }
    let heights = heights.clone();
    dom::set_timeout(50, move || scroll_to_item(viewport, &heights, ids, window, item_id, smooth, attempt + 1));
}
