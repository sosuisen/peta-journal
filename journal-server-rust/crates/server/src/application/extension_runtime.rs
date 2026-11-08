//! Runs the `main.js` of an extension function with QuickJS (see ADR 009).

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{ExtensionEvent, ExtensionRuntime, ItemRequest, PageInfo};
    use crate::application::flat_markdown_splitter::FlatMarkdownSplitter;
    use crate::application::markdown_parser::MarkdownParser;
    use crate::domain::id_generator::IdGenerator;
    use crate::domain::model::ItemPosition;

    fn runtime() -> ExtensionRuntime {
        ExtensionRuntime::new(
            Arc::new(FlatMarkdownSplitter::new(MarkdownParser)),
            Arc::new(IdGenerator::new()),
        )
    }

    fn function_dir(main_js: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.js"), main_js).unwrap();
        dir
    }

    fn page() -> PageInfo {
        PageInfo {
            id: "page-1".to_string(),
            file_name: "2026-11-08".to_string(),
            dir_path: "journals/".to_string(),
            page_type: "journal".to_string(),
        }
    }

    // onPageCreateはページの情報を受け取り、createItemで頼んだ項目が操作として返る。
    #[test]
    fn on_page_create_gets_the_page_and_returns_the_items_it_asks_for() {
        let dir = function_dir(
            "globalThis.onPageCreate = (page) => {
                peta.createItem({ position: 'last', level: 1, body: page.fileName + ' ' + page.pageType });
            };",
        );

        let requests = runtime()
            .run(dir.path(), &ExtensionEvent::PageCreate(page()))
            .unwrap();

        assert_eq!(requests.len(), 1);
        let ItemRequest {
            page_id,
            position,
            level,
            body,
            transclude,
            ..
        } = &requests[0];
        assert_eq!(page_id, "page-1");
        assert_eq!(*position, ItemPosition::Last);
        assert_eq!(*level, 1);
        assert_eq!(body, "2026-11-08 journal");
        assert_eq!(*transclude, None);
    }
}
