//! The move of an atlas page from the form the server wrote to a plain page (ADR 053).

use std::collections::BTreeMap;

use hmtid::Hmtid;
use peta_atlas::PAGE_KEY;

use super::model::Item;

/// What changes in the items of an atlas page when it moves to the form of a plain page.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Migration {
    /// The scalar properties that go, by the id of their item.
    pub dropped_props: Vec<(Hmtid, &'static str)>,
}

/// What moves the items of an atlas page to the form of a plain page, from the items in
/// their order and their scalar properties by item id. None when nothing is to move.
pub fn migration(
    items: &[Item],
    props: &BTreeMap<String, BTreeMap<String, String>>,
) -> Option<Migration> {
    let mut migration = Migration::default();
    for item in items {
        let has = |key| {
            props
                .get(item.id.as_str())
                .is_some_and(|p| p.contains_key(key))
        };
        if has(PAGE_KEY) {
            migration.dropped_props.push((item.id.clone(), PAGE_KEY));
        }
    }
    (migration != Migration::default()).then_some(migration)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u32) -> Hmtid {
        Hmtid::parse(&format!("2026-09-29-00-00-{n:02}-AAAAAAA")).unwrap()
    }

    fn item(n: u32, level: i32, body: &str) -> Item {
        Item {
            id: id(n),
            page_id: id(59),
            parent_item_id: None,
            dir_path: "d/".to_string(),
            name: "index_atlas".to_string(),
            item_type: "doc".to_string(),
            created_date: 0,
            modified_date: 0,
            idx: format!("{n:04}"),
            level,
            body: body.to_string(),
            collapsed: false,
            types: Vec::new(),
            transclude_item_id: None,
        }
    }

    type Props = BTreeMap<String, BTreeMap<String, String>>;

    fn props(entries: &[(u32, &str, &str)]) -> Props {
        let mut props = Props::new();
        for (n, key, value) in entries {
            props
                .entry(id(*n).to_string())
                .or_default()
                .insert(key.to_string(), value.to_string());
        }
        props
    }

    // ページのカードからatlas_pageが消える。座標のないカードも、項目は残る。
    #[test]
    fn a_page_card_loses_its_atlas_page() {
        let items = [item(1, 1, "[[d/a]]"), item(2, 1, "[[d/b]]")];
        let props = props(&[
            (1, "atlas_page", "p1"),
            (1, "atlas_bounds", "0,0,0,40"),
            (2, "atlas_page", "p2"),
        ]);

        let migration = migration(&items, &props).unwrap();

        assert_eq!(
            migration,
            Migration {
                dropped_props: vec![(id(1), "atlas_page"), (id(2), "atlas_page")],
            }
        );
    }

    // グループからatlas_typeが消える。atlas_sideと、グループの下のカードのレベルは変わらない。
    #[test]
    fn a_group_loses_its_atlas_type() {
        let items = [item(1, 1, "Title"), item(2, 2, "[[x/a]]")];
        let props = props(&[
            (1, "atlas_type", "group"),
            (1, "atlas_side", "left"),
            (2, "atlas_bounds", "0,0,0,40"),
        ]);

        let migration = migration(&items, &props).unwrap();

        assert_eq!(
            migration,
            Migration {
                dropped_props: vec![(id(1), "atlas_type")],
            }
        );
    }
}
