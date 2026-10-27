//! The canvas of an atlas page: the pages and folders of its directory as cards, drawn by
//! PixiJS in `atlas.js`.

use crate::core::CoreHandle;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/src/components/atlas.js")]
extern "C" {
    type AtlasCanvas;

    #[wasm_bindgen(constructor)]
    fn new(host: &web_sys::HtmlElement) -> AtlasCanvas;

    #[wasm_bindgen(method, js_name = setCards)]
    fn set_cards(this: &AtlasCanvas, labels: Vec<String>);

    #[wasm_bindgen(method)]
    fn destroy(this: &AtlasCanvas);
}

#[component]
pub fn Atlas() -> impl IntoView {
    let core: CoreHandle = expect_context();
    // The sidebar has already loaded the directory of the atlas page as its index list.
    // Its labels name a page without its directory and a folder with a trailing slash.
    let labels = core.select(|v| {
        v.sidebar
            .index_pages
            .iter()
            .map(|p| p.label.clone())
            .collect::<Vec<_>>()
    });
    let host = NodeRef::<leptos::html::Div>::new();
    let canvas = StoredValue::new_local(None::<AtlasCanvas>);

    Effect::new(move |_| {
        let labels = labels.get();
        let Some(el) = host.get() else {
            return;
        };
        canvas.update_value(|c| {
            c.get_or_insert_with(|| AtlasCanvas::new(&el)).set_cards(labels);
        });
    });
    on_cleanup(move || {
        canvas.try_update_value(|c| {
            if let Some(c) = c.take() {
                c.destroy();
            }
        });
    });

    view! { <div class="atlas-canvas" node_ref=host></div> }
}
