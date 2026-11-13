//! The outline, ported from `outline.ts`: a windowed list of rows (only the rows near the
//! viewport are in the DOM, the rest is padding measured from the rows' real heights), the
//! scroll reports the journal needs, scrolling to an item, and the document listeners of
//! selection mode.

use crate::components::confirm_dialog;
use crate::components::edge_scroll::EdgeScroll;
use crate::components::outline_item::OutlineItem;
use crate::components::wiki_suggestions::WikiSuggestions;
use crate::core::CoreHandle;
use crate::dom;
use crate::listeners::DocumentListeners;
use leptos::prelude::*;
use peta_core::app::SelectionKey;
use peta_core::view::Row;
use peta_core::Event;
use std::collections::{HashMap, HashSet};
use wasm_bindgen::JsCast;

/// Pixels rendered above and below the viewport, like the CDK's min/max buffer.
const MIN_BUFFER_PX: f64 = 500.0;
const MAX_BUFFER_PX: f64 = 1000.0;
/// The height of a row that was never measured.
const ESTIMATED_ROW_PX: f64 = 28.0;
/// The share of the viewport height left empty below the last row.
const BOTTOM_ROOM_RATIO: f64 = 0.5;

type ViewportRef = NodeRef<leptos::html::Custom<&'static str>>;

#[derive(Clone, Copy, PartialEq, Debug, Default)]
struct Window {
    start: usize,
    end: usize,
    pad_top: f64,
    pad_bottom: f64,
}

/// Rows by id with one signal each, so a change to one item re-renders one row.
#[derive(Clone, Copy)]
struct RowStore {
    ids: RwSignal<Vec<String>>,
    rows: StoredValue<HashMap<String, RwSignal<Row>>>,
}

impl RowStore {
    fn new(vm: RwSignal<peta_core::ViewModel>) -> RowStore {
        let store = RowStore {
            ids: RwSignal::new(Vec::new()),
            rows: StoredValue::new(HashMap::new()),
        };
        // Row signals are made under the outline's owner, not the effect's: an effect disposes
        // what was created in its previous run
        let owner = Owner::current().expect("an owner");
        Effect::new(move |_| {
            vm.with(|v| {
                let mut new_ids = Vec::with_capacity(v.outline.rows.len());
                store.rows.update_value(|map| {
                    for row in &v.outline.rows {
                        new_ids.push(row.item.id.clone());
                        match map.get(&row.item.id) {
                            Some(sig) => {
                                if sig.with_untracked(|r| r != row) {
                                    sig.set(row.clone());
                                }
                            }
                            None => {
                                let sig = owner.with(|| RwSignal::new(row.clone()));
                                map.insert(row.item.id.clone(), sig);
                            }
                        }
                    }
                    let keep: HashSet<&String> = new_ids.iter().collect();
                    map.retain(|id, _| keep.contains(id));
                });
                if store.ids.with_untracked(|cur| *cur != new_ids) {
                    store.ids.set(new_ids);
                }
            });
        });
        store
    }

    fn row(&self, id: &str) -> Option<RwSignal<Row>> {
        self.rows.with_value(|rows| rows.get(id).copied())
    }
}

/// The windowed list: which rows are in the DOM and how much padding stands in for the rest.
/// Everything in it is `Copy`, so the scroll handler, the effects and the timers share it.
#[derive(Clone, Copy)]
struct Windowing {
    viewport: ViewportRef,
    wrapper: NodeRef<leptos::html::Div>,
    /// The block above the rows (the page properties); it scrolls with them.
    header: NodeRef<leptos::html::Div>,
    ids: RwSignal<Vec<String>>,
    /// The measured height of each row, by item id.
    heights: StoredValue<HashMap<String, f64>>,
    window: RwSignal<Window>,
    measure_scheduled: StoredValue<bool>,
}

impl Windowing {
    /// False once the outline left the screen; its timers and frames then do nothing.
    fn alive(self) -> bool {
        !self.window.is_disposed()
    }

    /// The height of the block above the rows; zero without one.
    fn header_height(self) -> f64 {
        self.header
            .get_untracked()
            .map(|el| el.offset_height() as f64)
            .unwrap_or(0.0)
    }

    fn height_of(heights: &HashMap<String, f64>, id: &str) -> f64 {
        heights.get(id).copied().unwrap_or(ESTIMATED_ROW_PX)
    }

    /// Recomputes the rendered range from the scroll position and the known heights.
    fn recompute(self) {
        if !self.alive() {
            return;
        }
        let Some(vp) = self.viewport.get_untracked() else {
            return;
        };
        // Row offsets start below the header
        let scroll_top = vp.scroll_top() as f64 - self.header_height();
        let vp_height = vp.client_height() as f64;
        let w = self.heights.with_value(|heights| {
            self.ids.with_untracked(|ids| {
                let top_edge = scroll_top - MIN_BUFFER_PX;
                let bottom_edge = scroll_top + vp_height + MAX_BUFFER_PX;
                let mut y = 0.0;
                let mut start = ids.len();
                let mut end = ids.len();
                let mut pad_top = 0.0;
                let mut pad_bottom = 0.0;
                for (i, id) in ids.iter().enumerate() {
                    let h = Self::height_of(heights, id);
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
                    let last = ids.last().map(|id| Self::height_of(heights, id));
                    pad_top = (y - last.unwrap_or(0.0)).max(0.0);
                    end = ids.len();
                    pad_bottom = 0.0;
                }
                // Half a viewport of room below the last row, so the bottommost item can
                // be scrolled up to the middle of the screen instead of sitting at its
                // bottom edge while being typed in
                Window {
                    start,
                    end,
                    pad_top,
                    pad_bottom: pad_bottom + vp_height * BOTTOM_ROOM_RATIO,
                }
            })
        });
        if self.window.get_untracked() != w {
            self.window.set(w);
        }
    }

    /// Measures the rendered rows on the next frame; a change re-runs the window computation.
    fn schedule_measure(self) {
        if self.measure_scheduled.get_value() {
            return;
        }
        self.measure_scheduled.set_value(true);
        request_animation_frame(move || {
            if !self.alive() {
                return;
            }
            self.measure_scheduled.set_value(false);
            if self.measure_rendered_rows() {
                self.recompute();
            }
        });
    }

    /// Reads the height of every rendered row; true when one changed.
    fn measure_rendered_rows(self) -> bool {
        let Some(wrap) = self.wrapper.get_untracked() else {
            return false;
        };
        let children = wrap.children();
        self.heights.try_update_value(|heights| {
            let mut changed = false;
            for i in 0..children.length() {
                let Some(el) = children
                    .item(i)
                    .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
                else {
                    continue;
                };
                let Some(id) = el.get_attribute("data-row-id") else {
                    continue;
                };
                let h = el.offset_height() as f64;
                let known = heights.get(&id).copied();
                if h > 0.0 && known.map(|old| (old - h).abs() > 0.5).unwrap_or(true) {
                    heights.insert(id, h);
                    changed = true;
                }
            }
            changed
        }) == Some(true)
    }

    /// The first rendered row whose bottom is below the viewport top.
    fn top_visible_index(self, scroll_top: f64) -> usize {
        let scroll_top = scroll_top - self.header_height();
        let w = self.window.get_untracked();
        self.heights.with_value(|heights| {
            self.ids.with_untracked(|ids| {
                let mut y = w.pad_top;
                for (i, id) in ids.iter().enumerate().skip(w.start).take(w.end - w.start) {
                    y += Self::height_of(heights, id);
                    if y > scroll_top {
                        return i;
                    }
                }
                w.start
            })
        })
    }

    /// Scrolls the item to a tenth from the top of the viewport. An item that is not rendered
    /// yet is approached by its estimated offset and then step by step until it appears.
    /// Scrolls to the item once its row is in the list. The rows of a page that just loaded
    /// may come a moment after the request.
    fn scroll_to_item_when_listed(self, item_id: String, smooth: bool, wait: u32) {
        if !self.alive() {
            return;
        }
        if self.ids.with_untracked(|ids| ids.contains(&item_id)) {
            self.scroll_to_item(item_id, smooth, 0);
        } else if wait < 20 {
            dom::set_timeout(50, move || {
                self.scroll_to_item_when_listed(item_id, smooth, wait + 1)
            });
        }
    }

    fn scroll_to_item(self, item_id: String, smooth: bool, attempt: u32) {
        if !self.alive() {
            return;
        }
        let Some(vp) = self.viewport.get_untracked() else {
            return;
        };
        if let Some(el) = dom::element_by_id(&item_id) {
            let el_rect = el.get_bounding_client_rect();
            let vp_rect = vp.get_bounding_client_rect();
            let target_y = vp_rect.top() + vp_rect.height() * 0.1;
            let options = web_sys::ScrollToOptions::new();
            options.set_top(el_rect.top() - target_y);
            options.set_behavior(if smooth || attempt > 0 {
                web_sys::ScrollBehavior::Smooth
            } else {
                web_sys::ScrollBehavior::Auto
            });
            vp.scroll_by_with_scroll_to_options(&options);
            return;
        }
        if attempt > 50 {
            return;
        }
        let Some(index) = self
            .ids
            .with_untracked(|ids| ids.iter().position(|id| *id == item_id))
        else {
            return;
        };
        let vp_height = vp.client_height() as f64;
        if attempt == 0 {
            // Estimate the position and jump
            let estimated = self.heights.with_value(|heights| {
                self.ids.with_untracked(|ids| {
                    ids.iter()
                        .take(index)
                        .map(|id| Self::height_of(heights, id))
                        .sum::<f64>()
                })
            });
            let estimated = estimated + self.header_height();
            vp.set_scroll_top((estimated - vp_height * 0.1).max(0.0) as i32);
        } else {
            let w = self.window.get_untracked();
            if index >= w.end {
                vp.set_scroll_top(vp.scroll_top() + (vp_height * 0.8) as i32);
            } else if index < w.start {
                vp.set_scroll_top((vp.scroll_top() - (vp_height * 0.8) as i32).max(0));
            }
        }
        dom::set_timeout(50, move || {
            self.scroll_to_item(item_id, smooth, attempt + 1)
        });
    }
}

/// `header` is rendered above the rows and `footer` below them, inside the scrolling
/// viewport, so they scroll with the rows.
#[component]
pub fn Outline(
    #[prop(optional)] header: Option<ViewFn>,
    #[prop(optional)] footer: Option<ViewFn>,
) -> impl IntoView {
    let core: CoreHandle = expect_context();
    let vm = core.view();
    let store = RowStore::new(vm);
    let list = Windowing {
        viewport: NodeRef::new(),
        wrapper: NodeRef::new(),
        header: NodeRef::new(),
        ids: store.ids,
        heights: StoredValue::new(HashMap::new()),
        window: RwSignal::new(Window::default()),
        measure_scheduled: StoredValue::new(false),
    };
    let window = list.window;

    // The window follows the rows and the scroll position
    Effect::new(move |_| {
        list.ids.track();
        list.recompute();
        list.schedule_measure();
    });
    Effect::new(move |_| {
        window.track();
        list.schedule_measure();
    });

    // Scroll: the top item and the buffer the journal keeps ahead
    let on_scroll = move |_| {
        list.recompute();
        let Some(vp) = list.viewport.get_untracked() else {
            return;
        };
        let scroll_top = vp.scroll_top() as f64;
        let w = window.get_untracked();
        core.send(Event::OutlineScrolled {
            scrolled_from_top: scroll_top > 0.0,
            top_visible_index: list.top_visible_index(scroll_top),
            rendered_start: w.start,
            rendered_end: w.end,
        });
    };

    // Scrolling to an item the sidebar or a search result asked for
    core.set_scroller(Some(Box::new(move |item_id: String, smooth: bool| {
        list.scroll_to_item_when_listed(item_id, smooth, 0);
    })));
    on_cleanup(move || core.set_scroller(None));

    // Selection mode: the keys and clicks of the whole document act on the selection, and a
    // drag held at an edge of the viewport scrolls it
    let selection_listeners = DocumentListeners::new();
    let edge_scroll = EdgeScroll::new(core, list.viewport);
    // A memo: the view model changes on every scroll, and an effect on it would stop the edge
    // scroll on its own first frame
    let selection_mode = Memo::new(move |_| vm.with(|v| v.outline.selection_mode));
    Effect::new(move |_| {
        selection_listeners.clear();
        edge_scroll.stop();
        if !selection_mode.get() {
            return;
        }
        selection_listeners.on("mousemove", move |ev: &web_sys::MouseEvent| {
            if ev.buttons() & 1 == 1 {
                edge_scroll.follow(ev.client_x() as f64, ev.client_y() as f64);
            } else {
                edge_scroll.stop();
            }
        });
        selection_listeners.on_cancelable("keydown", move |ev: &web_sys::KeyboardEvent| {
            if confirm_dialog::is_inside(ev) {
                return;
            }
            if let Some(key) = selection_key(ev) {
                ev.prevent_default();
                send_selection_key(core, key);
            }
        });
        selection_listeners.on("click", move |ev: &web_sys::Event| {
            if !confirm_dialog::is_inside(ev) {
                core.send(Event::SelectionClickOutside)
            }
        });
        selection_listeners.on("mouseup", move |_: &web_sys::Event| {
            edge_scroll.stop();
            core.send(Event::SelectionMouseUp)
        });
        for (name, key) in [("copy", SelectionKey::Copy), ("cut", SelectionKey::Cut)] {
            selection_listeners.on_cancelable(name, move |ev: &web_sys::Event| {
                ev.prevent_default();
                send_selection_key(core, key);
            });
        }
    });

    let rendered_ids = move || {
        let w = window.get();
        list.ids.with(|ids| {
            ids.iter()
                .skip(w.start)
                .take(w.end.saturating_sub(w.start))
                .cloned()
                .collect::<Vec<_>>()
        })
    };

    view! {
        <app-outline>
            <cdk-virtual-scroll-viewport node_ref=list.viewport class="outline-viewport" on:scroll=on_scroll>
                {header.map(|h| view! { <div node_ref=list.header class="outline-header">{h.run()}</div> })}
                <div
                    node_ref=list.wrapper
                    class="cdk-virtual-scroll-content-wrapper"
                    style:padding-top=move || format!("{}px", window.get().pad_top)
                    style:padding-bottom=move || format!("{}px", window.get().pad_bottom)
                >
                    <For
                        each=rendered_ids
                        key=|id| id.clone()
                        children=move |id| {
                            let Some(row) = store.row(&id) else { return ().into_any() };
                            view! { <OutlineRow id=id row=row /> }.into_any()
                        }
                    />
                </div>
                {footer.map(|f| view! { <div class="outline-footer">{f.run()}</div> })}
            </cdk-virtual-scroll-viewport>
            <WikiSuggestions />
        </app-outline>
    }
}

/// One row of the windowed list: the date header of a journal day and the item.
#[component]
fn OutlineRow(id: String, row: RwSignal<Row>) -> impl IntoView {
    let core: CoreHandle = expect_context();
    let item_id = StoredValue::new(id.clone());
    let on_wheel = move |ev: web_sys::WheelEvent| {
        if !ev.shift_key() {
            return;
        }
        ev.prevent_default();
        core.send(Event::ZoomStep {
            item_id: item_id.get_value(),
            up: ev.delta_y() < 0.0,
        });
    };
    let date_header = move || {
        row.with(|r| r.date_header.clone())
            .map(|h| view! { <div class="date-header">{h}</div> })
    };
    view! {
        <div
            class="outline-row"
            data-row-id=id
            on:mouseenter=move |_| core.send(Event::ItemMouseEnter { item_id: item_id.get_value() })
            on:wheel=on_wheel
        >
            {date_header}
            <OutlineItem row=row />
        </div>
    }
}

/// Sends a selection key to the core. A copy or cut starts its clipboard write here, while
/// the browser still counts the key or copy event as the user's gesture.
fn send_selection_key(core: CoreHandle, key: SelectionKey) {
    if matches!(key, SelectionKey::Copy | SelectionKey::Cut) {
        crate::effects::ui::begin_copy(core);
    }
    core.send(Event::SelectionKey(key));
}

fn selection_key(ev: &web_sys::KeyboardEvent) -> Option<SelectionKey> {
    let key = ev.key();
    if key == "Escape" {
        return Some(SelectionKey::Escape);
    }
    if (key == "z" || key == "Z") && ev.ctrl_key() && !ev.alt_key() {
        return Some(if ev.shift_key() {
            SelectionKey::Redo
        } else {
            SelectionKey::Undo
        });
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
        return Some(if ev.shift_key() {
            SelectionKey::Outdent
        } else {
            SelectionKey::Indent
        });
    }
    // Cmd+C/X on macOS: handled here and not left to the copy event, which Safari does not
    // dispatch unless some text is selected
    let copy_modifier = ev.ctrl_key() || ev.meta_key();
    if copy_modifier && key == "c" {
        return Some(SelectionKey::Copy);
    }
    if copy_modifier && key == "x" {
        return Some(SelectionKey::Cut);
    }
    None
}
