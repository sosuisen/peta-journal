//! The move of the atlas pages from the form the server wrote to plain pages (ADR 053).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use hmtid::Hmtid;
use journal_core::domain::model::ItemPosition;
use journal_core::infrastructure::export::import_service::Outcome;
use journal_core::infrastructure::persistence::PropertyRepository;

use crate::common::devices::{time_passes, two_devices, Device, ATLAS};

impl Device {
    /// An item at the end of the atlas with the atlas properties `props`, as the server
    /// wrote one before the atlas became a plain page
    fn old_item(&self, level: i32, body: &str, props: &[(&str, &str)]) -> Hmtid {
        let item = self
            .core
            .item_service
            .create_item(&self.atlas_id(), &ItemPosition::Last, level, body, None)
            .unwrap();
        self.give_atlas_props(&item.id, props);
        item.id
    }

    /// Gives an item atlas properties that no client can set, as the server wrote them
    fn give_atlas_props(&self, item_id: &Hmtid, props: &[(&str, &str)]) {
        let conn = self.core.items.db().conn().unwrap();
        for (key, value) in props {
            PropertyRepository::write_scalar_in(&conn, "item", item_id.as_str(), key, value)
                .unwrap();
        }
    }

    /// The atlas properties of an item of the atlas
    fn atlas_props(&self, item_id: &Hmtid) -> BTreeMap<String, String> {
        self.core
            .properties
            .find_scalar_by_targets("item", &[item_id.to_string()])
            .unwrap()
            .remove(item_id.as_str())
            .unwrap_or_default()
    }

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

/// Two devices that share the directory `d/`. On A, its atlas is as the server wrote it
/// before the atlas became a plain page: the card of the page a.
fn with_an_old_atlas(dir: &Path) -> (Device, Device) {
    let (a, b) = two_devices(dir);
    let page = a.page_in("d/", "a").unwrap();
    a.old_item(1, "[[d/a]]", &[("atlas_page", page.as_str())]);
    // The server wrote no empty item into an atlas
    let items = a.core.page_service.get_page_items(&a.atlas_id()).unwrap();
    a.core.item_service.delete_item(&items[0].id).unwrap();
    (a, b)
}

// 古い形のアトラスを移すと、ページのカードからatlas_pageが消える。項目は残る。
#[test]
fn an_atlas_of_the_old_form_moves_to_the_plain_form() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b) = with_an_old_atlas(dir.path());
    assert_eq!(a.atlas_keys(), BTreeSet::from(["atlas_page".to_string()]));

    a.core.atlas.move_to_plain_form().unwrap();

    assert_eq!(a.atlas_keys(), BTreeSet::new());
    assert_eq!(a.outline(), ["1:[[d/a]]"]);
}

fn props(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

// 古い形のアトラスを移すと、グループからatlas_typeが消える。コールアウトの親は消え、コールアウトがレベル1に上がる。グループは子の座標の重心を自分の座標として得て、子の座標はそこからの相対になる。ほかのプロパティは変わらない。
#[test]
fn the_groups_and_the_callouts_of_an_old_atlas_move_to_the_plain_form() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b) = with_an_old_atlas(dir.path());
    let group = a.old_item(
        1,
        "Title",
        &[("atlas_type", "group"), ("atlas_side", "left")],
    );
    let first = a.old_item(2, "[[x/b]]", &[("atlas_bounds", "100,10,0,40")]);
    let second = a.old_item(2, "[[x/c]]", &[("atlas_bounds", "300,50,0,60")]);
    a.old_item(1, "callout", &[("atlas_type", "callouts")]);
    let callout = [
        ("atlas_type", "callout"),
        ("atlas_bounds", "9,9,0,40"),
        ("atlas_side", "top"),
    ];
    let hello = a.old_item(2, "Hello", &callout);

    a.core.atlas.move_to_plain_form().unwrap();

    assert_eq!(
        a.outline(),
        ["1:[[d/a]]", "1:Title", "2:[[x/b]]", "2:[[x/c]]", "1:Hello"]
    );
    assert_eq!(
        a.atlas_props(&group),
        props(&[("atlas_side", "left"), ("atlas_bounds", "200,30,0,40")])
    );
    assert_eq!(
        a.atlas_props(&first),
        props(&[("atlas_bounds", "-100,-20,0,40")])
    );
    assert_eq!(
        a.atlas_props(&second),
        props(&[("atlas_bounds", "100,20,0,60")])
    );
    assert_eq!(a.atlas_props(&hello), props(&callout));
}

// すでに新しい形に移したアトラスでも、`atlas_bounds`のない親は、座標のある子の重心を座標として得る。もう一度移しても、何も変わらない。
#[test]
fn a_parent_without_bounds_of_a_plain_atlas_gets_bounds_once() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b) = two_devices(dir.path());
    let parent = a.old_item(1, "Title", &[]);
    let card = a.old_item(2, "[[x/b]]", &[("atlas_bounds", "30,40,0,40")]);
    let alone = a.old_item(1, "Alone", &[]);
    let waiting = a.old_item(2, "[[x/c]]", &[]);

    a.core.atlas.move_to_plain_form().unwrap();

    let moved =
        |device: &Device| [&parent, &card, &alone, &waiting].map(|id| device.atlas_props(id));
    let once = moved(&a);
    assert_eq!(
        once,
        [
            props(&[("atlas_bounds", "30,40,0,40")]),
            props(&[("atlas_bounds", "0,0,0,40")]),
            props(&[]),
            props(&[]),
        ]
    );

    a.core.atlas.move_to_plain_form().unwrap();

    assert_eq!(moved(&a), once);
    assert_eq!(a.conflicts().len(), 1);
}

// 移したアトラスのページの修正日付は、移した時刻になる。移すもののないアトラスの日付は変わらない。
#[test]
fn a_moved_atlas_is_dated_when_it_moved() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b) = with_an_old_atlas(dir.path());
    let modified = |device: &Device| {
        let atlas = device.core.pages.find_by_id(&device.atlas_id());
        atlas.unwrap().unwrap().modified_date
    };
    let before = modified(&a);

    time_passes(&[&a]);
    a.core.atlas.move_to_plain_form().unwrap();

    // The clock of a device runs on, so the date is told from the ones around it
    let moved = modified(&a);
    assert!(before < moved && moved <= a.core.items.db().now_millis());

    time_passes(&[&a]);
    a.core.atlas.move_to_plain_form().unwrap();

    assert_eq!(modified(&a), moved);
}

// 移す直前のindex_atlas.mdが、同じフォルダーにバックアップとして残る。移すもののないアトラスには、バックアップを作らない。
#[test]
fn the_file_of_an_atlas_is_kept_as_a_backup_before_it_moves() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b) = with_an_old_atlas(dir.path());
    let old = a.file(ATLAS);
    assert!(old.contains("atlas_page"), "{old}");

    a.core.atlas.move_to_plain_form().unwrap();

    let backups = a.conflicts();
    assert_eq!(backups.len(), 1, "{backups:?}");
    assert_eq!(std::fs::read_to_string(&backups[0]).unwrap(), old);

    a.core.atlas.move_to_plain_form().unwrap();

    assert_eq!(a.conflicts(), backups);
}

// 起動のときに、古い形のアトラスが新しい形に移る。
#[test]
fn an_old_atlas_moves_when_the_core_starts() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b) = with_an_old_atlas(dir.path());
    a.old_item(1, "Title", &[("atlas_type", "group")]);
    a.old_item(2, "[[x/b]]", &[("atlas_bounds", "0,0,0,40")]);
    a.old_item(2, "[[x/c]]", &[("atlas_bounds", "9,9,0,40")]);

    a.core.start_up();

    assert_eq!(a.atlas_keys(), BTreeSet::from(["atlas_bounds".to_string()]));
    assert_eq!(
        a.outline(),
        ["1:[[d/a]]", "1:Title", "2:[[x/b]]", "2:[[x/c]]"]
    );
}

// 取り込んだindex_atlas.mdが古い形なら、新しい形に移してから取り込まれる。
#[test]
fn an_old_atlas_taken_in_from_a_file_moves_to_the_plain_form() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = with_an_old_atlas(dir.path());
    a.old_item(1, "Title", &[("atlas_type", "group")]);
    a.old_item(2, "[[x/b]]", &[("atlas_bounds", "0,0,0,40")]);

    b.receive(&a, ATLAS);

    assert_eq!(b.atlas_keys(), BTreeSet::from(["atlas_bounds".to_string()]));
    assert_eq!(b.outline(), ["1:[[d/a]]", "1:Title", "2:[[x/b]]"]);
}

// 2台が別々に古い形のアトラスを移してから同期すると、中身が同じになり、それ以上は送り合わない。
#[test]
fn two_devices_that_moved_an_atlas_each_agree_and_send_it_no_more() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = with_an_old_atlas(dir.path());
    a.old_item(1, "Title", &[("atlas_type", "group")]);
    a.old_item(2, "[[x/b]]", &[("atlas_bounds", "30,40,0,40")]);
    a.old_item(2, "[[x/c]]", &[("atlas_bounds", "50,40,0,40")]);
    a.old_item(1, "callout", &[("atlas_type", "callouts")]);
    a.old_item(2, "Hello", &[("atlas_type", "callout")]);
    // B takes in the old form and moves it; A moves its own later
    b.receive(&a, ATLAS);
    time_passes(&[&a, &b]);
    a.core.atlas.move_to_plain_form().unwrap();

    b.receive(&a, ATLAS);
    a.receive(&b, ATLAS);

    assert_eq!(
        a.outline(),
        ["1:[[d/a]]", "1:Title", "2:[[x/b]]", "2:[[x/c]]", "1:Hello"]
    );
    assert!(a.file(ATLAS).contains("40,40,0,40"), "{}", a.file(ATLAS));
    assert_eq!(b.outline(), a.outline());
    assert_eq!(a.file(ATLAS), b.file(ATLAS));
    assert_eq!(b.receive(&a, ATLAS), Outcome::Unchanged);
    assert_eq!(a.receive(&b, ATLAS), Outcome::Unchanged);
}
