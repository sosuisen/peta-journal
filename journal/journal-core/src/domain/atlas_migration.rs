//! The move of an atlas page from the form the server wrote to a plain page (ADR 053).

use std::collections::BTreeMap;

use hmtid::Hmtid;
use peta_atlas::{Bounds, AUTO_WIDTH, BOUNDS_KEY, CARD_HEIGHT};
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
    // The items that stay, as the parents are given their bounds from them
    let mut staying: Vec<Placed> = Vec::new();
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
        let level = match under_callouts {
            Some(_) => item.level - 1,
            None => item.level,
        };
        if under_callouts.is_some() {
            migration.levels.push((item.id.clone(), level));
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
        if prop(TYPE_KEY) != Some(CALLOUTS_TYPE) {
            let stored = prop(BOUNDS_KEY).and_then(Bounds::parse);
            staying.push(Placed {
                id: &item.id,
                level,
                stored,
                bounds: stored,
            });
        }
    }
    migration.bounds = bounds_of_parents(staying);
    (migration != Migration::default()).then_some(migration)
}

/// An item of the plain form with its `atlas_bounds`: as they are stored, and as they are once
/// the parents have theirs.
struct Placed<'a> {
    id: &'a Hmtid,
    level: i32,
    stored: Option<Bounds>,
    bounds: Option<Bounds>,
}

/// The `atlas_bounds` to write so that every parent of items with bounds has bounds of its
/// own (ADR 053). The bounds of an item are counted from its parent, but the old form counted
/// them from the center of the canvas and gave a parent none. Such a parent gets the centroid
/// of the items right under it that have bounds, rounded to whole numbers, and those items
/// are counted from it, so nothing moves on the canvas. An inner parent comes before the
/// outer one, and counts for the outer one by the bounds it got. A parent that has bounds and
/// the items under it stay as they are.
fn bounds_of_parents(mut items: Vec<Placed>) -> Vec<(Hmtid, String)> {
    // From the last item up, so every item under a parent is settled before the parent
    for at in (0..items.len()).rev() {
        if items[at].bounds.is_some() {
            continue;
        }
        let children = placed_children(&items, at);
        if children.is_empty() {
            continue;
        }
        let mean = |of: fn(&Bounds) -> f64| {
            let places = children.iter().filter_map(|&i| items[i].bounds.as_ref());
            (places.map(of).sum::<f64>() / children.len() as f64).round()
        };
        let (x, y) = (mean(|b| b.x), mean(|b| b.y));
        for &i in &children {
            items[i].bounds = items[i].bounds.map(|b| b.moved_by(-x, -y));
        }
        items[at].bounds = Some(Bounds {
            x,
            y,
            width: AUTO_WIDTH,
            height: CARD_HEIGHT,
        });
    }
    items
        .iter()
        .filter(|item| item.bounds != item.stored)
        .filter_map(|item| Some((item.id.clone(), item.bounds?.to_string())))
        .collect()
}

/// The indexes of the items right under the item at `at` that have bounds. An item is right
/// under it when no item between them has a lower level than its own.
fn placed_children(items: &[Placed], at: usize) -> Vec<usize> {
    let mut children = Vec::new();
    // The lowest level seen under the parent so far: a deeper item is under one of those
    let mut lowest = i32::MAX;
    for (i, below) in items.iter().enumerate().skip(at + 1) {
        if below.level <= items[at].level {
            break;
        }
        if below.level <= lowest {
            lowest = below.level;
            if below.bounds.is_some() {
                children.push(i);
            }
        }
    }
    children
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

    // グループからatlas_typeが消える。atlas_sideと、グループの下のカードのレベルは変わらない。グループは、子の座標を自分の座標として得る。
    #[test]
    fn a_group_loses_its_atlas_type() {
        let items = [item(1, 1, "Title"), item(2, 2, "[[x/a]]")];
        let props = props(&[
            (1, "atlas_type", "group"),
            (1, "atlas_side", "left"),
            (2, "atlas_bounds", "5,6,0,40"),
        ]);

        let migration = migration(&items, &props).unwrap();

        assert_eq!(
            migration,
            Migration {
                dropped_props: vec![(id(1), "atlas_type")],
                bounds: vec![
                    (id(1), "5,6,0,40".to_string()),
                    (id(2), "0,0,0,40".to_string())
                ],
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

    // 入れ子の親は、内側から先に決まる。内側の親は、決まった自分の座標で、外側の親の子の1つとして数えられる。
    #[test]
    fn an_inner_parent_is_settled_first_and_counts_for_the_outer_one_by_its_own_place() {
        let items = [
            item(1, 1, "outer"),
            item(2, 2, "inner"),
            item(3, 3, "[[x/a]]"),
            item(4, 3, "[[x/b]]"),
            item(5, 2, "[[x/c]]"),
        ];
        let props = props(&[
            (3, "atlas_bounds", "100,100,0,40"),
            (4, "atlas_bounds", "300,100,0,40"),
            (5, "atlas_bounds", "0,0,0,40"),
        ]);

        let migration = migration(&items, &props).unwrap();

        // The inner one lies at 200,100 on the canvas, and the outer one at 100,50
        assert_eq!(
            migration.bounds,
            [
                (id(1), "100,50,0,40".to_string()),
                (id(2), "100,50,0,40".to_string()),
                (id(3), "-100,0,0,40".to_string()),
                (id(4), "100,0,0,40".to_string()),
                (id(5), "-100,-50,0,40".to_string()),
            ]
        );
    }

    // 重心は整数に丸められる。丸めてから引くので、親と子の座標を足すと、元の座標に戻る。動かない子は書き直されない。
    #[test]
    fn the_centroid_is_rounded_before_the_children_are_counted_from_it() {
        let items = [
            item(1, 1, "Title"),
            item(2, 2, "[[x/a]]"),
            item(3, 2, "[[x/b]]"),
            item(4, 1, "Still"),
            item(5, 2, "[[x/c]]"),
        ];
        let props = props(&[
            (2, "atlas_bounds", "0,0.5,0,40"),
            (3, "atlas_bounds", "3,0.5,0,40"),
            (5, "atlas_bounds", "0,0.25,0,40"),
        ]);

        let migration = migration(&items, &props).unwrap();

        assert_eq!(
            migration.bounds,
            [
                (id(1), "2,1,0,40".to_string()),
                (id(2), "-2,-0.5,0,40".to_string()),
                (id(3), "1,-0.5,0,40".to_string()),
                (id(4), "0,0,0,40".to_string()),
            ]
        );
    }

    // `atlas_bounds`を持つ親と、その下の項目は変わらない。座標のある子を持たない親には、足さない。消えるコールアウトの親にも足さない。
    #[test]
    fn a_parent_with_bounds_stays_and_one_without_a_placed_child_gets_none() {
        let items = [
            item(1, 1, "placed"),
            item(2, 2, "[[x/a]]"),
            item(3, 1, "empty"),
            item(4, 2, "[[x/b]]"),
            item(5, 3, "[[x/c]]"),
            item(6, 1, "callout"),
            item(7, 2, "Hello"),
        ];
        let props = props(&[
            (1, "atlas_bounds", "50,50,0,40"),
            (2, "atlas_bounds", "10,10,0,40"),
            (6, "atlas_type", "callouts"),
            (7, "atlas_type", "callout"),
            (7, "atlas_bounds", "9,9,0,40"),
        ]);

        let migration = migration(&items, &props).unwrap();

        assert_eq!(migration.bounds, []);
    }
}
