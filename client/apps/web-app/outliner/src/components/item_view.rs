//! The rendered item, ported from `item-view.ts`: the AST as HTML, syntax highlighting, and
//! clicks that open links or start editing at the clicked character.

use crate::components::item_drag::Drag;
use crate::components::item_html::{item_html, ImageSize};
use crate::components::task_checkbox::TaskCheckbox;
use crate::core::CoreHandle;
use crate::dom;
use client_core::view::Row;
use client_core::Event;
use client_page_links::{hashtag_path, wikilink_path};
use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// The marks that `ast_to_html` puts on a wikilink and on a hashtag, as CSS selectors.
const WIKILINKS: &str = "a[data-wikilink]";
const HASHTAGS: &str = "a.hashtag";
/// The attribute that keeps the name of the page a wikilink or a hashtag leads to.
const PAGE_OF_LINK: &str = "data-page";

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
    let html = Memo::new(move |_| row.with(|r| item_html(&r.item.ast)));
    // A hashtag links under the configured tag directory, so a tag page can live in its own
    // folder (e.g. `tags/topic`) and a published site can keep the same relative paths.
    let tags_dir = core.select(|v| v.config.tags_dir.clone());

    // What `ast_to_html` leaves to the shell, once the HTML is in the page (ADR 026). The
    // view may be gone by then (replaced by the editor, scrolled out of the window), so the
    // node ref is read with `try_`: a disposed ref is not an error, just nothing to do.
    Effect::new(move |_| {
        html.track();
        let tags_dir = tags_dir.get();
        dom::after_render(move || {
            if let Some(el) = content.try_get_untracked().flatten() {
                point_links_at_pages(&el, &tags_dir);
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
            let cursor = match content.get() {
                Some(el) => {
                    let cursor =
                        dom::text_offset_at_point(&el, ev.client_x() as f64, ev.client_y() as f64);
                    drag.start(core, &id, &el, Some(cursor));
                    cursor
                }
                None => 0,
            };
            core.send(Event::EditRequested {
                item_id: id.clone(),
                cursor,
            });
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
                let new_tab = ev.ctrl_key() || ev.meta_key();
                let new_window = ev.shift_key();
                let href = link.get_attribute("href");
                if new_tab && !new_window && href.is_some() {
                    return; // the browser opens it in a background tab
                }
                ev.prevent_default();
                ev.stop_propagation();
                let url = href.unwrap_or_else(|| format!("/pages/Untitled-{}", untitled_suffix()));
                if new_window || new_tab {
                    let _ = dom::window().open_with_url_and_target_and_features(
                        &url,
                        "_blank",
                        if new_window { "noopener" } else { "" },
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

fn untitled_suffix() -> String {
    let d = js_sys::Date::new_0();
    format!(
        "{:04}{:02}{:02}{:02}{:02}",
        d.get_full_year(),
        d.get_month() + 1,
        d.get_date(),
        d.get_hours(),
        d.get_minutes()
    )
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

/// Makes every wikilink and hashtag point at its page. `ast_to_html` writes the name of the
/// page alone, which the browser would read as relative to the page that is open. The browser
/// uses the `href` as it is for Ctrl+click, the middle button and its own menu.
fn point_links_at_pages(content: &web_sys::Element, tags_dir: &str) {
    point_links(content, WIKILINKS, wikilink_path);
    point_links(content, HASHTAGS, |tag| hashtag_path(tags_dir, tag));
}

/// Gives every link that `selector` finds the path that `path_of` makes from its name.
fn point_links(content: &web_sys::Element, selector: &str, path_of: impl Fn(&str) -> String) {
    for link in elements(content, selector) {
        // The name is kept apart from the `href`: this runs again on the same links when
        // the tag directory changes, and must not add a second prefix.
        let name = link.get_attribute(PAGE_OF_LINK).unwrap_or_else(|| {
            let name = link.get_attribute("href").unwrap_or_default();
            let _ = link.set_attribute(PAGE_OF_LINK, &name);
            name
        });
        let _ = link.set_attribute("href", &path_of(&name));
    }
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
