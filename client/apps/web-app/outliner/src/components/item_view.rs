//! The rendered item, ported from `item-view.ts`: the AST as HTML, syntax highlighting, and
//! clicks that open links or start editing at the clicked character.

use crate::body::Row;
use crate::components::item_drag::Drag;
use crate::components::item_html::{item_html, ImageSize};
use crate::components::task_checkbox::TaskCheckbox;
use crate::core::CoreHandle;
use crate::dom;
use client_core::keys::OpenIn;
use client_core::util::date::generate_untitled_title;
use client_core::{EditPlace, Event};
use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// The marks that `ast_to_html_with` puts on a wikilink and on a hashtag, as CSS selectors.
const WIKILINKS: &str = "a[data-wikilink]";
const HASHTAGS: &str = "a.hashtag";

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = hljs, js_name = highlightElement, catch)]
    fn hljs_highlight_element(el: &web_sys::Element) -> Result<(), JsValue>;
}

#[component]
pub fn ItemView(row: RwSignal<Row>) -> impl IntoView {
    let core: CoreHandle = expect_context();
    let drag: Drag = expect_context();
    let item_id = row.with_untracked(|r| r.item.id.clone());
    let content = NodeRef::<leptos::html::Div>::new();
    // A hashtag links under the configured tag directory, so a tag page can live in its own
    // folder (e.g. `tags/topic`). The HTML is made again when the directory changes.
    let tags_dir = core.select(|v| v.config.tags_dir.clone());
    let html = Memo::new(move |_| {
        let tags_dir = tags_dir.get();
        row.with(|r| item_html(&r.body_model.0, &tags_dir))
    });

    // What `ast_to_html_with` leaves to the shell, once the HTML is in the page (ADR 026). The
    // view may be gone by then (replaced by the editor, scrolled out of the window), so the
    // node ref is read with `try_`: a disposed ref is not an error, just nothing to do.
    Effect::new(move |_| {
        html.track();
        dom::after_render(move || {
            if let Some(el) = content.try_get_untracked().flatten() {
                size_images(&el);
                highlight_code_blocks(&el);
            }
        });
    });

    // Start editing on mousedown; links are left alone
    let on_mousedown = {
        let id = item_id.clone();
        move |ev: web_sys::MouseEvent| {
            if dom::target_element(&ev).is_none() || dom::target_closest(&ev, "a").is_some() {
                return;
            }
            // The editor replaces this view and takes the focus inside this very handler; the
            // browser's own focus change on mousedown would take it away again
            ev.prevent_default();
            let el = content.get();
            // The core lines up the letters of the page before the click with the Markdown,
            // and puts the cursor there (ADR 044). Without letters it goes to the end (ADR 057)
            let before = el.as_ref().and_then(|el| {
                dom::text_before_point(el, ev.client_x() as f64, ev.client_y() as f64)
            });
            let place = match before {
                Some(before) => EditPlace::AfterShownText(before),
                None => EditPlace::Unplaced,
            };
            core.send(Event::EditRequested {
                item_id: id.clone(),
                place,
            });
            if let Some(el) = el {
                // The drag selects from the place where the core put the cursor
                let cursor = core.view().with_untracked(|v| {
                    v.outline
                        .focus_request
                        .as_ref()
                        .filter(|r| r.item_id == id)
                        .and_then(|r| r.cursor)
                });
                drag.start(core, &id, &el, cursor);
            }
        }
    };

    // An image dropped on a rendered item is appended to its body. The drop point is not
    // mapped to a character: the rendered HTML and the markdown do not line up.
    let on_drop = {
        let id = item_id.clone();
        move |ev: web_sys::DragEvent| {
            let Some(file) = ev.data_transfer().and_then(|d| dom::image_file(&d)) else {
                return;
            };
            ev.prevent_default();
            let cursor = row.with_untracked(|r| r.item.body.chars().count());
            let handle = core.keep_file(file);
            core.send(Event::PasteImage {
                item_id: id.clone(),
                file: handle,
                cursor,
            });
        }
    };

    // Link clicks (Ctrl+click on a wiki link stays native, so it opens a background tab)
    let on_click = {
        move |ev: web_sys::MouseEvent| {
            let Some(target) = dom::target_element(&ev) else {
                return;
            };
            if let Some(footnote) = target
                .closest(".footnote-ref a, .footnote-label")
                .ok()
                .flatten()
            {
                ev.prevent_default();
                ev.stop_propagation();
                if let Some(id) = footnote
                    .get_attribute("href")
                    .and_then(|h| h.strip_prefix('#').map(str::to_string))
                {
                    if let Some(el) = dom::element_by_id(&id) {
                        let options = web_sys::ScrollIntoViewOptions::new();
                        options.set_block(web_sys::ScrollLogicalPosition::Center);
                        el.scroll_into_view_with_scroll_into_view_options(&options);
                    }
                }
                return;
            }
            if let Some(anchor) = target
                .closest(&format!("a:not({WIKILINKS}):not({HASHTAGS})"))
                .ok()
                .flatten()
            {
                let anchor: web_sys::HtmlAnchorElement = anchor.unchecked_into();
                let href = anchor.href();
                if !href.is_empty() {
                    ev.prevent_default();
                    ev.stop_propagation();
                    let _ = if ev.shift_key() {
                        dom::window()
                            .open_with_url_and_target_and_features(&href, "_blank", "noopener")
                    } else {
                        dom::window().open_with_url_and_target(&href, "_blank")
                    };
                    return;
                }
            }
            if let Some(link) = target
                .closest(&format!("{WIKILINKS}, {HASHTAGS}"))
                .ok()
                .flatten()
            {
                let open_in = dom::held(&ev).open_in();
                let href = link.get_attribute("href");
                if open_in == OpenIn::Tab && href.is_some() {
                    return; // the browser opens it in a background tab
                }
                ev.prevent_default();
                ev.stop_propagation();
                let url = href.unwrap_or_else(|| {
                    page_urls::page_url(&generate_untitled_title(dom::local_now().into()))
                });
                if open_in != OpenIn::Here {
                    let _ = dom::window().open_with_url_and_target_and_features(
                        &url,
                        "_blank",
                        if open_in == OpenIn::Window {
                            "noopener"
                        } else {
                            ""
                        },
                    );
                } else {
                    core.navigate(&url, false);
                }
            }
        }
    };

    view! {
        <app-item-view>
            <TaskCheckbox row=row />
            <div
                node_ref=content
                class="item-content"
                inner_html=move || html.get()
                on:mousedown=on_mousedown
                on:click=on_click
                on:drop=on_drop
            ></div>
        </app-item-view>
    }
}

/// The elements under `content` that `selector` finds.
fn elements(content: &web_sys::Element, selector: &str) -> Vec<web_sys::HtmlElement> {
    let Ok(found) = content.query_selector_all(selector) else {
        return Vec::new();
    };
    (0..found.length())
        .filter_map(|i| found.get(i)?.dyn_into().ok())
        .collect()
}

/// Gives every image the size that its URL carries (see `item_html`).
fn size_images(content: &web_sys::Element) {
    for image in elements(content, "img") {
        let size = ImageSize::of_src(&image.get_attribute("src").unwrap_or_default());
        for (side, pixels) in [("width", size.width), ("height", size.height)] {
            if let Some(pixels) = pixels {
                let _ = image.set_attribute(side, &pixels.to_string());
            }
        }
    }
}

/// Colours every code block that names its language, and shows the name on it. A block
/// without a language is left alone: highlight.js would guess one and colour plain text.
fn highlight_code_blocks(content: &web_sys::Element) {
    for code in elements(content, "pre code") {
        let Some(lang) = code
            .class_name()
            .split(' ')
            .find_map(|c| c.strip_prefix("language-").map(str::to_string))
        else {
            continue;
        };
        let Some(pre) = code
            .parent_element()
            .and_then(|p| p.dyn_into::<web_sys::HtmlElement>().ok())
        else {
            continue;
        };
        let _ = hljs_highlight_element(&code);
        if pre
            .query_selector(".code-lang-label")
            .ok()
            .flatten()
            .is_none()
        {
            if let Ok(label) = dom::document().create_element("div") {
                label.set_class_name("code-lang-label");
                label.set_text_content(Some(&lang));
                let _ = pre.style().set_property("position", "relative");
                let _ = pre.append_child(&label);
            }
        }
    }
}
