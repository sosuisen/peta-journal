//! The move of the atlas pages from the form the server wrote to plain pages (ADR 053).

use std::collections::BTreeSet;

use crate::common::devices::{two_devices, Device};

impl Device {
    /// The keys of the atlas properties that the items of the atlas have
    fn atlas_keys(&self) -> BTreeSet<String> {
        let items = self
            .core
            .page_service
            .get_page_items(&self.atlas_id())
            .unwrap();
        let ids: Vec<String> = items.iter().map(|i| i.id.to_string()).collect();
        self.core
            .properties
            .find_scalar_by_targets("item", &ids)
            .unwrap()
            .into_values()
            .flat_map(|props| props.into_keys())
            .collect()
    }
}

// 古い形のアトラスを移すと、ページのカードからatlas_pageが消える。項目は残る。
#[test]
fn an_atlas_of_the_old_form_moves_to_the_plain_form() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b) = two_devices(dir.path());
    assert_eq!(a.atlas_keys(), BTreeSet::from(["atlas_page".to_string()]));

    a.core.atlas.move_to_plain_form().unwrap();

    assert_eq!(a.atlas_keys(), BTreeSet::new());
    assert_eq!(a.outline(), ["1:[[d/a]]"]);
}
