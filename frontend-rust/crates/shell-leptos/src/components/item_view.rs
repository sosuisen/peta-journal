//! The rendered item, ported from `item-view.ts`: the AST as HTML, syntax highlighting, and
//! clicks that open links or start editing at the clicked character.

use crate::components::task_checkbox::TaskCheckbox;
use crate::core::CoreHandle;
use crate::dom;
use fm_leptos::{FmRoot, RenderOptions};
use leptos::prelude::*;
use peta_core::view::Row;
use peta_core::Event;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = hljs, js_name = highlightElement, catch)]
    fn hljs_highlight_element(el: &web_sys::Element) -> Result<(), JsValue>;
}

#[component]
pub fn ItemView(row: RwSignal<Row>) -> impl IntoView {
    let core: CoreHandle = expect_context();
    let item_id = row.with_untracked(|r| r.item.id.clone());
    let content = NodeRef::<leptos::html::Div>::new();
    let ast = Memo::new(move |_| row.with(|r| r.item.ast.clone()));

    // Syntax highlighting and the language label, once the AST is rendered. The view may be
    // gone by then (replaced by the editor, scrolled out of the window), so the node ref is
    // read with `try_`: a disposed ref is not an error, just nothing to highlight.
    Effect::new(move |_| {
        ast.track();
        dom::after_render(move || {
            if let Some(el) = content.try_get_untracked().flatten() {
                highlight_code_blocks(&el);
            }
        });
    });

    // Start editing on mousedown; links are left alone
    let on_mousedown = {
        let id = item_id.clone();
        move |ev: web_sys::MouseEvent| {
            let Some(target) = ev
                .target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            else {
                return;
            };
            if target.closest("a").ok().flatten().is_some() {
                return;
            }
            // The editor replaces this view and takes the focus inside this very handler; the
            // browser's own focus change on mousedown would take it away again
            ev.prevent_default();
            let cursor = match content.get() {
                Some(el) => {
                    dom::text_offset_at_point(&el, ev.client_x() as f64, ev.client_y() as f64)
                }
                None => 0,
            };
            core.send(Event::EditRequested {
                item_id: id.clone(),
                cursor,
            });
        }
    };

    // Link clicks (Ctrl+click on a wiki link stays native, so it opens a background tab)
    let on_click = {
        move |ev: web_sys::MouseEvent| {
            let Some(target) = ev
                .target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            else {
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
                .closest("a:not(.wiki-link):not(.wiki-link-new):not(.hashtag-link)")
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
                .closest(".wiki-link, .wiki-link-new, .hashtag-link")
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

    let options = RenderOptions {
        wikilink_prefix: "/pages/".into(),
        wikilink_class: "wiki-link".into(),
        hashtag_prefix: "/pages/".into(),
        hashtag_class: "hashtag-link".into(),
    };

    view! {
        <app-item-view>
            <TaskCheckbox row=row />
            <div node_ref=content class="item-content" on:mousedown=on_mousedown on:click=on_click>
                {move || {
                    let options = options.clone();
                    view! { <FmRoot ast=ast.get() options=options /> }
                }}
            </div>
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

fn highlight_code_blocks(content: &web_sys::HtmlDivElement) {
    let Ok(blocks) = content.query_selector_all("pre code") else {
        return;
    };
    for i in 0..blocks.length() {
        let Some(code) = blocks
            .get(i)
            .and_then(|n| n.dyn_into::<web_sys::HtmlElement>().ok())
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
            let lang = code
                .class_name()
                .split(' ')
                .find_map(|c| c.strip_prefix("language-").map(str::to_string));
            if let Some(lang) = lang {
                if let Ok(label) = dom::document().create_element("div") {
                    label.set_class_name("code-lang-label");
                    label.set_text_content(Some(&lang));
                    let _ = pre.style().set_property("position", "relative");
                    let _ = pre.append_child(&label);
                }
            }
        }
    }
}
