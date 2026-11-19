//! The runtime of a build without the `extensions` feature: it runs no script.

use std::path::Path;
use std::sync::Arc;

use crate::application::flat_markdown_splitter::FlatMarkdownSplitter;
use crate::domain::id_generator::IdGenerator;

use super::{Confirmer, ExtensionEvent, ExtensionLimits, ItemRequest};

/// Has the methods of the QuickJS runtime and does nothing with what it is given.
pub struct ExtensionRuntime;

impl ExtensionRuntime {
    pub fn new(_splitter: Arc<FlatMarkdownSplitter>, _ids: Arc<IdGenerator>) -> ExtensionRuntime {
        ExtensionRuntime
    }

    pub fn with_limits(self, _limits: ExtensionLimits) -> ExtensionRuntime {
        self
    }

    pub fn with_confirmer(self, _confirmer: Arc<dyn Confirmer>) -> ExtensionRuntime {
        self
    }

    /// Asks for no item: the script in `dir` is not read.
    pub fn run(&self, _dir: &Path, _event: &ExtensionEvent) -> anyhow::Result<Vec<ItemRequest>> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::super::{ExtensionEvent, PageInfo};
    use super::ExtensionRuntime;
    use crate::application::flat_markdown_splitter::FlatMarkdownSplitter;
    use crate::application::markdown_parser::MarkdownParser;
    use crate::domain::id_generator::IdGenerator;

    // 拡張関数のないビルドでは、スクリプトがあっても項目は頼まれない。
    #[test]
    fn a_script_asks_for_no_item_in_a_build_without_extensions() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("main.js"),
            "globalThis.onPageCreate = () => {
                peta.createItem({ position: 'last', level: 1, body: 'x' });
            };",
        )
        .unwrap();
        let runtime = ExtensionRuntime::new(
            Arc::new(FlatMarkdownSplitter::new(MarkdownParser)),
            Arc::new(IdGenerator::default()),
        );
        let page = PageInfo {
            id: "page-1".to_string(),
            file_name: "2026-11-08".to_string(),
            dir_path: "journals/".to_string(),
            page_type: "journal".to_string(),
            journal: None,
        };

        let requests = runtime
            .run(dir.path(), &ExtensionEvent::PageCreate(page))
            .unwrap();

        assert!(requests.is_empty());
    }
}
