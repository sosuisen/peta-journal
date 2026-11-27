//! The move of an atlas page from the form the server wrote to a plain page (ADR 053).

use std::collections::BTreeMap;

use hmtid::Hmtid;
use peta_atlas::{CALLOUTS_TYPE, GROUP_TYPE, PAGE_KEY, TYPE_KEY};

use super::model::Item;

/// What changes in the items of an atlas page when it moves to the form of a plain page.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Migration {
    /// The scalar properties that go, by the id of their item.
    pub dropped_props: Vec<(Hmtid, &'static str)>,
    /// The items that go.
    pub removed: Vec<Hmtid>,
    /// The new levels of the items whose level changes.
    pub levels: Vec<(Hmtid, i32)>,
    /// The `atlas_bounds` that are written, by the id of their item.
    pub bounds: Vec<(Hmtid, String)>,
}

/// What moves the items of an atlas page to the form of a plain page, from the items in
/// their order and their scalar properties by item id. None when nothing is to move.
pub fn migration(
    items: &[Item],
    props: &BTreeMap<String, BTreeMap<String, String>>,
) -> Option<Migration> {
    let mut migration = Migration::default();
    // The level of the parent of the callouts whose items are passed now
    let mut under_callouts: Option<i32> = None;
    for item in items {
        let prop = |key| {
            props
                .get(item.id.as_str())
                .and_then(|p| p.get(key))
                .map(String::as_str)
        };
        if under_callouts.is_some_and(|parent| item.level <= parent) {
            under_callouts = None;
        }
        if under_callouts.is_some() {
            migration.levels.push((item.id.clone(), item.level - 1));
        }
        if prop(PAGE_KEY).is_some() {
            migration.dropped_props.push((item.id.clone(), PAGE_KEY));
        }
        match prop(TYPE_KEY) {
            Some(GROUP_TYPE) => migration.dropped_props.push((item.id.clone(), TYPE_KEY)),
            Some(CALLOUTS_TYPE) => {
                migration.removed.push(item.id.clone());
                under_callouts = Some(item.level);
            }
            _ => {}
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
                ..Migration::default()
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
                ..Migration::default()
            }
        );
    }

    // コールアウトの親が消え、子のコールアウトがレベル1に上がる。コールアウトのプロパティは変わらない。
    #[test]
    fn the_parent_of_the_callouts_goes_and_the_callouts_come_up() {
        let items = [
            item(1, 1, "[[d/a]]"),
            item(2, 1, "callout"),
            item(3, 2, "Hello"),
            item(4, 2, "World"),
        ];
        let props = props(&[
            (2, "atlas_type", "callouts"),
            (3, "atlas_type", "callout"),
            (3, "atlas_side", "left"),
            (4, "atlas_type", "callout"),
        ]);

        let migration = migration(&items, &props).unwrap();

        assert_eq!(
            migration,
            Migration {
                removed: vec![id(2)],
                levels: vec![(id(3), 1), (id(4), 1)],
                ..Migration::default()
            }
        );
    }

    // 新しい形の項目には、移すものがない。カード、子を持つ項目、レベル1のコールアウトである。
    #[test]
    fn nothing_moves_in_an_atlas_of_the_plain_form() {
        let items = [
            item(1, 1, "[[d/a]]"),
            item(2, 1, "Title"),
            item(3, 2, "[[x/b]]"),
            item(4, 1, "Hello"),
        ];
        let props = props(&[
            (1, "atlas_bounds", "0,0,0,40"),
            (2, "atlas_side", "left"),
            (4, "atlas_type", "callout"),
            (4, "atlas_bounds", "9,9,0,40"),
        ]);

        assert_eq!(migration(&items, &props), None);
    }

    // `atlas_bounds`のない親は、座標のある子の座標の重心を、自分の座標として得る。子の座標は、親からの相対に書き直され、見える位置は変わらない。座標のない子は数えない。
    #[test]
    fn a_parent_without_bounds_gets_the_centroid_of_its_children_and_they_are_counted_from_it() {
        let items = [
            item(1, 1, "Title"),
            item(2, 2, "[[x/a]]"),
            item(3, 2, "[[x/b]]"),
            item(4, 2, "[[x/c]]"),
            item(5, 1, "[[x/d]]"),
        ];
        let props = props(&[
            (2, "atlas_bounds", "100,10,0,40"),
            (3, "atlas_bounds", "300,50,120,60"),
            (5, "atlas_bounds", "7,8,0,40"),
        ]);

        let migration = migration(&items, &props).unwrap();

        assert_eq!(
            migration,
            Migration {
                bounds: vec![
                    (id(1), "200,30,0,40".to_string()),
                    (id(2), "-100,-20,0,40".to_string()),
                    (id(3), "100,20,120,60".to_string()),
                ],
                ..Migration::default()
            }
        );
    }
}
