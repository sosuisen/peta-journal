//! A file taken in item by item: the later change of each item stays, so that edits of
//! different items of a page on two devices are both kept. See docs/folder-sync-future.md.

use std::path::Path;

use hmtid::Hmtid;
use journal_server::domain::model::ItemPosition;
use journal_server::infrastructure::export::import_service::Outcome;

use super::atlas_sync::{two_devices, Device, ATLAS};

const PAGE: &str = "docs/d/a.md";

impl Device {
    /// Adds an item at the end of the page, as a new line in the app
    fn add(&self, page_id: &Hmtid, body: &str) -> Hmtid {
        self.state
            .item_service
            .create_item(page_id, &ItemPosition::Last, 1, body, None)
            .unwrap()
            .id
    }

    fn write(&self, item_id: &Hmtid, body: &str) {
        self.state
            .item_service
            .update_item(item_id, Some(body), None, None)
            .unwrap();
    }

    fn delete(&self, item_id: &Hmtid) {
        self.state.item_service.delete_item(item_id).unwrap();
    }

    fn bodies(&self, page_id: &Hmtid) -> Vec<String> {
        self.state
            .page_service
            .get_page_items(page_id)
            .unwrap()
            .into_iter()
            .map(|i| i.body)
            .filter(|b| !b.is_empty())
            .collect()
    }
}

/// Two devices that both hold the items x and y of the page a
fn two_devices_with_items(dir: &Path) -> (Device, Device, Hmtid, Hmtid, Hmtid) {
    let (a, b) = two_devices(dir);
    let page = a.page_in("d/", "a").unwrap();
    let x = a.add(&page, "x");
    let y = a.add(&page, "y");
    b.receive(&a, PAGE);
    (a, b, page, x, y)
}

/// The `modified` value on the metadata line of the item
fn modified_of(file: &str, item_id: &Hmtid) -> Option<String> {
    let line = file.lines().find(|l| l.contains(item_id.as_str()))?;
    let (_, rest) = line.split_once(" modified=\"")?;
    Some(rest.split('"').next()?.to_string())
}

/// The `modified` value of the front matter
fn page_date_of(file: &str) -> Option<String> {
    let line = file.lines().find(|l| l.starts_with("modified: "))?;
    Some(line.trim_start_matches("modified: ").to_string())
}

/// The file as the server wrote it before the items had dates
fn without_item_dates(file: &str) -> String {
    let mut out = String::new();
    for line in file.split_inclusive('\n') {
        match line.find(" modified=\"") {
            Some(start) if line.starts_with("<!--") => {
                let rest = &line[start + " modified=\"".len()..];
                let end = rest.find('"').unwrap();
                out.push_str(&line[..start]);
                out.push_str(&rest[end + 1..]);
            }
            _ => out.push_str(line),
        }
    }
    out
}

// 同じページの別の項目を2台で編集しても、両方の編集が残る。
#[test]
fn edits_of_different_items_on_two_devices_are_both_kept() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, x, y) = two_devices_with_items(dir.path());

    a.write(&x, "x from A");
    b.write(&y, "y from B");
    assert_eq!(b.receive(&a, PAGE), Outcome::Imported);
    assert_eq!(a.receive(&b, PAGE), Outcome::Imported);

    assert_eq!(a.bodies(&page), ["x from A", "y from B"]);
    assert_eq!(b.bodies(&page), ["x from A", "y from B"]);
    assert_eq!(a.file(PAGE), b.file(PAGE));
    assert!(a.conflicts().is_empty() && b.conflicts().is_empty());
}

// 古いファイルが後から届いても、別の項目の編集は両方残る。
#[test]
fn an_older_file_brings_its_edit_and_leaves_the_later_edit_of_another_item() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, x, y) = two_devices_with_items(dir.path());

    a.write(&x, "x from A");
    a.export();
    b.write(&y, "y from B");
    assert_eq!(a.receive(&b, PAGE), Outcome::Imported);
    assert_eq!(b.receive(&a, PAGE), Outcome::Imported);

    assert_eq!(a.bodies(&page), ["x from A", "y from B"]);
    assert_eq!(b.bodies(&page), ["x from A", "y from B"]);
}

// 同じ項目を2台で編集すると、後の編集が両方に残る。
#[test]
fn the_later_edit_of_one_item_wins_on_both_devices() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, x, _) = two_devices_with_items(dir.path());

    a.write(&x, "x from A");
    a.export();
    b.write(&x, "x from B");
    a.receive(&b, PAGE);
    b.receive(&a, PAGE);

    assert_eq!(a.bodies(&page), ["x from B", "y"]);
    assert_eq!(b.bodies(&page), ["x from B", "y"]);
}

// 前回の同期の後に相手が編集した項目が負けると、そのファイルはバックアップに残る。
#[test]
fn a_file_whose_edit_loses_is_kept_as_a_backup() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, x, _) = two_devices_with_items(dir.path());
    a.write(&x, "x from A");
    a.export();
    b.write(&x, "x from B");
    b.deliver(&a, PAGE);

    assert_eq!(b.sync(PAGE), Outcome::Conflict);

    assert_eq!(b.bodies(&page), ["x from B", "y"]);
    let backups = b.conflicts();
    assert_eq!(backups.len(), 1);
    assert!(std::fs::read_to_string(&backups[0])
        .unwrap()
        .contains("x from A"));
}

// プロパティの変更も項目の変更であり、古いファイルに負けない。
#[test]
fn a_later_property_is_a_change_of_the_item_and_stays_against_an_older_file() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, _, _) = two_devices_with_items(dir.path());
    a.edit(&page, "from A");
    a.export();
    b.edit(&page, "from B");
    b.deliver(&a, PAGE);

    assert_eq!(b.sync(PAGE), Outcome::Conflict);

    assert!(b.file(PAGE).contains("from B"));
}

// 新しいファイルに無くても、それより後に足した項目は残り、相手にも届く。
#[test]
fn an_item_added_after_the_file_was_written_stays_and_reaches_the_other_device() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, x, _) = two_devices_with_items(dir.path());
    a.write(&x, "x from A");
    a.export();
    b.add(&page, "z from B");

    b.receive(&a, PAGE);
    a.receive(&b, PAGE);

    assert_eq!(a.bodies(&page), ["x from A", "y", "z from B"]);
    assert_eq!(b.bodies(&page), ["x from A", "y", "z from B"]);
}

// ここより新しいファイルにあれば、ここに無い項目は足される。
#[test]
fn an_item_only_in_a_newer_file_is_added() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, x, y) = two_devices_with_items(dir.path());

    b.add(&page, "z from B");
    a.write(&x, "x from A");
    b.write(&y, "y from B");
    a.receive(&b, PAGE);

    assert_eq!(a.bodies(&page), ["x from A", "y from B", "z from B"]);
}

// ここより古いファイルにしかない項目でも、前回の同期の後にできたものなら足される。
#[test]
fn an_item_made_after_the_last_sync_comes_in_from_an_older_file() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, x, _) = two_devices_with_items(dir.path());
    b.add(&page, "z from B");
    b.export();
    a.write(&x, "x from A");

    assert_eq!(a.receive(&b, PAGE), Outcome::Imported);

    assert_eq!(a.bodies(&page), ["x from A", "y", "z from B"]);
    assert!(a.conflicts().is_empty());
}

// まだ書き出していない項目は、より新しいファイルに無くても残る。
#[test]
fn an_item_not_written_yet_stays_against_a_newer_file_without_it() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, x, _) = two_devices_with_items(dir.path());
    b.add(&page, "z from B");
    a.write(&x, "x from A");

    b.receive(&a, PAGE);

    assert_eq!(b.bodies(&page), ["x from A", "y", "z from B"]);
}

// 2台で同時に足した項目は、どちらも両方に残る。
#[test]
fn items_added_on_two_devices_at_once_are_both_kept_on_both() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, _, _) = two_devices_with_items(dir.path());
    a.add(&page, "from A");
    b.add(&page, "from B");
    a.export();

    b.receive(&a, PAGE);
    a.receive(&b, PAGE);

    let mut on_a = a.bodies(&page);
    on_a.sort();
    assert_eq!(on_a, ["from A", "from B", "x", "y"]);
    assert_eq!(a.bodies(&page), b.bodies(&page));
    assert_eq!(a.file(PAGE), b.file(PAGE));
}

// ファイルを書いたあとで消した項目は、古いファイルが届いても戻らない。
#[test]
fn an_item_deleted_after_the_file_was_written_does_not_come_back() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, x, y) = two_devices_with_items(dir.path());
    a.write(&x, "x from A");
    a.export();
    b.delete(&y);

    b.receive(&a, PAGE);

    assert_eq!(b.bodies(&page), ["x from A"]);
    assert!(b.conflicts().is_empty());
}

// 相手が消した項目は、それより後に変えていなければ消える。
#[test]
fn an_item_deleted_on_the_other_device_goes() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, _, y) = two_devices_with_items(dir.path());

    a.delete(&y);
    b.receive(&a, PAGE);

    assert_eq!(b.bodies(&page), ["x"]);
    assert!(b.conflicts().is_empty());
}

// 相手が消した項目でも、それより後に編集していれば両方に残る。
#[test]
fn an_item_edited_after_it_was_deleted_on_the_other_device_stays_on_both() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, _, y) = two_devices_with_items(dir.path());
    a.delete(&y);
    a.export();
    b.write(&y, "y from B");

    b.receive(&a, PAGE);
    a.receive(&b, PAGE);

    assert_eq!(a.bodies(&page), ["x", "y from B"]);
    assert_eq!(b.bodies(&page), ["x", "y from B"]);
}

// 手で書き換えたファイルは取り込まれ、その項目の日付は新しくなる。
#[test]
fn a_file_edited_by_hand_is_taken_in_and_the_item_gets_a_new_date() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b, page, x, _) = two_devices_with_items(dir.path());
    let before = a.file(PAGE);
    let edited = before.replace("\nx\n", "\nx by hand\n");
    assert_ne!(before, edited);
    std::fs::write(a.root.join(PAGE), edited).unwrap();

    assert_eq!(a.sync(PAGE), Outcome::Imported);

    assert_eq!(a.bodies(&page), ["x by hand", "y"]);
    let after = a.file(PAGE);
    assert!(after.contains("x by hand"));
    assert_ne!(modified_of(&before, &x), modified_of(&after, &x));
}

// 古い内容のまま手で保存したファイルは、アプリの編集に負けてもバックアップに残る。
#[test]
fn a_file_saved_by_hand_from_old_content_is_kept_as_a_backup_when_it_loses() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b, page, x, _) = two_devices_with_items(dir.path());
    let before = a.file(PAGE);
    a.write(&x, "x in the app");
    a.export();
    let edited = before.replace("\nx\n", "\nx by hand\n");
    std::fs::write(a.root.join(PAGE), edited).unwrap();

    assert_eq!(a.sync(PAGE), Outcome::Conflict);

    assert_eq!(a.bodies(&page), ["x in the app", "y"]);
    let backups = a.conflicts();
    assert_eq!(backups.len(), 1);
    assert!(std::fs::read_to_string(&backups[0])
        .unwrap()
        .contains("x by hand"));
}

// 手で足した項目は、古い日付とidが書かれていても取り込まれる。
#[test]
fn an_item_added_by_hand_with_an_old_date_and_its_own_id_is_taken_in() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b, page, _, _) = two_devices_with_items(dir.path());
    let mut file = a.file(PAGE);
    file.push_str("\n<!-- @ 2020-01-01-00-00-00-AAAAAAA idx=\"a9\" modified=\"2020-01-01T00:00:00Z\" -->\nby hand\n");
    std::fs::write(a.root.join(PAGE), file).unwrap();

    assert_eq!(a.sync(PAGE), Outcome::Imported);

    assert_eq!(a.bodies(&page), ["x", "y", "by hand"]);
}

// ファイルより後に動かした項目は、その位置に残り、相手にも届く。
#[test]
fn an_item_moved_after_the_file_was_written_keeps_its_place_on_both_devices() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, x, y) = two_devices_with_items(dir.path());
    b.state
        .item_service
        .move_items(std::slice::from_ref(&y), &page, &ItemPosition::First, None)
        .unwrap();
    a.write(&x, "x from A");
    a.export();

    b.receive(&a, PAGE);
    a.receive(&b, PAGE);

    assert_eq!(b.bodies(&page), ["y", "x from A"]);
    assert_eq!(a.bodies(&page), ["y", "x from A"]);
}

// 内容が同じなら、ファイルの日付が古くてもバックアップは作らない。
#[test]
fn a_file_that_only_has_older_dates_makes_no_backup() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b, page, x, _) = two_devices_with_items(dir.path());
    a.write(&x, "x #tagged");
    a.edit(&page, "a note");
    let file = a.file(PAGE);
    a.write(&x, "x #tagged");
    a.export();
    std::fs::write(a.root.join(PAGE), file).unwrap();

    assert_eq!(a.sync(PAGE), Outcome::Imported);

    assert!(a.conflicts().is_empty());
}

// ファイルの並びが他の項目の位置を奪っても、残した項目は元の位置にとどまる。
#[test]
fn a_kept_item_stays_in_its_place_when_the_file_takes_the_index_of_another() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b, page, x, y) = two_devices_with_items(dir.path());
    let file = a.file(PAGE);
    let items = a.state.page_service.get_page_items(&page).unwrap();
    let idx_of = |id: &Hmtid| items.iter().find(|i| i.id == *id).unwrap().idx.clone();
    let (idx_x, idx_y) = (idx_of(&x), idx_of(&y));
    let swapped = file
        .replace(&format!("idx=\"{idx_x}\""), "idx=\"swap\"")
        .replace(&format!("idx=\"{idx_y}\""), &format!("idx=\"{idx_x}\""))
        .replace("idx=\"swap\"", &format!("idx=\"{idx_y}\""));
    a.state
        .item_service
        .create_item(&page, &ItemPosition::First, 1, "z", None)
        .unwrap();
    std::fs::write(a.root.join(PAGE), swapped).unwrap();

    a.sync(PAGE);

    assert_eq!(a.bodies(&page), ["z", "x", "y"]);
}

// 内容が同じでも、ファイルの日付が新しければ、項目はその日付になる。
#[test]
fn an_item_with_the_same_content_takes_the_later_date_of_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, x, _) = two_devices_with_items(dir.path());
    a.write(&x, "x");
    let from_a = a.file(PAGE);

    b.receive(&a, PAGE);

    let written = b.state.export.serialize_to_outline_markdown(
        &b.state.pages.find_by_id(&page).unwrap().unwrap(),
        &b.state.page_service.get_page_items(&page).unwrap(),
    );
    assert_eq!(modified_of(&written.unwrap(), &x), modified_of(&from_a, &x));
}

// 手で消した行の項目は消える。
#[test]
fn an_item_whose_lines_were_removed_by_hand_goes() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b, page, _, y) = two_devices_with_items(dir.path());
    let before = a.file(PAGE);
    let kept: Vec<&str> = before
        .split_inclusive('\n')
        .filter(|l| !l.contains(y.as_str()) && *l != "y\n")
        .collect();
    std::fs::write(a.root.join(PAGE), kept.concat()).unwrap();

    assert_eq!(a.sync(PAGE), Outcome::Imported);

    assert_eq!(a.bodies(&page), ["x"]);
}

// 項目の日付のない古い形のファイルでは、ページの日付で判断する。
#[test]
fn a_file_without_item_dates_counts_by_the_date_of_the_page() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, x, y) = two_devices_with_items(dir.path());
    let date_of_y = modified_of(&b.file(PAGE), &y);
    a.write(&x, "x from A");
    let old_form = without_item_dates(&a.file(PAGE));
    std::fs::write(b.root.join(PAGE), &old_form).unwrap();

    assert_eq!(b.sync(PAGE), Outcome::Imported);

    assert_eq!(b.bodies(&page), ["x from A", "y"]);
    let written = b.state.export.serialize_to_outline_markdown(
        &b.state.pages.find_by_id(&page).unwrap().unwrap(),
        &b.state.page_service.get_page_items(&page).unwrap(),
    );
    let written = written.unwrap();
    // The changed item takes the date of the file, and the other item keeps its own
    assert_eq!(modified_of(&written, &x), page_date_of(&old_form));
    assert_eq!(modified_of(&written, &y), date_of_y);
}

// ページのファイルの項目には更新日が書かれ、アトラスのファイルには書かれない。
#[test]
fn the_file_of_a_page_dates_its_items_and_the_file_of_an_atlas_does_not() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b, _, x, _) = two_devices_with_items(dir.path());

    assert!(modified_of(&a.file(PAGE), &x).is_some());
    assert!(!a.file(ATLAS).contains(" modified=\""));
}
