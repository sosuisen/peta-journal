//! A file taken in item by item: the later change of each item stays, so that edits of
//! different items of a page on two devices are both kept. See docs/folder-sync-future.md.

use std::path::Path;
use std::time::{Duration, SystemTime};

use hmtid::Hmtid;
use journal_core::domain::model::ItemPosition;
use journal_core::infrastructure::export::export_service::format_instant;
use journal_core::infrastructure::export::import_service::Outcome;
use regex::Regex;

use crate::common::devices::{time_passes, two_devices, Device, ATLAS};

const PAGE: &str = "docs/d/a.md";

impl Device {
    /// Adds an item at the end of the page, as a new line in the app
    fn add(&self, page_id: &Hmtid, body: &str) -> Hmtid {
        self.core
            .item_service
            .create_item(page_id, &ItemPosition::Last, 1, body, None)
            .unwrap()
            .id
    }

    fn write(&self, item_id: &Hmtid, body: &str) {
        self.core
            .item_service
            .update_item(item_id, Some(body), None, None)
            .unwrap();
    }

    fn delete(&self, item_id: &Hmtid) {
        self.core.item_service.delete_item(item_id).unwrap();
    }

    /// Gives the file a modification time, as the file system does when it writes the file
    fn date_file(&self, rel_path: &str, time: SystemTime) {
        let file = std::fs::File::options()
            .write(true)
            .open(self.root.join(rel_path))
            .unwrap();
        file.set_modified(time).unwrap();
    }

    fn item_idx(&self, item_id: &Hmtid) -> String {
        self.core.item_service.get_item_by_id(item_id).unwrap().idx
    }

    fn indexes(&self, page_id: &Hmtid) -> Vec<String> {
        self.core
            .page_service
            .get_page_items(page_id)
            .unwrap()
            .into_iter()
            .filter(|i| !i.body.is_empty())
            .map(|i| i.idx)
            .collect()
    }

    fn bodies(&self, page_id: &Hmtid) -> Vec<String> {
        self.core
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

/// The file with the date as the `modified` value of its page and of every item
fn dated(file: &str, millis: i64) -> String {
    let date = format_instant(millis);
    let file = Regex::new(r"(?m)^modified: .*$")
        .unwrap()
        .replace(file, format!("modified: {date}"));
    Regex::new(r#" modified="[^"]*""#)
        .unwrap()
        .replace_all(&file, format!(r#" modified="{date}""#))
        .into_owned()
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
    // An edit on B in the same millisecond as the export on A dates B's file at the last
    // sync of A, the date of a file edited by hand on A, which A takes as a conflict
    time_passes(&[&a, &b]);
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

// 定義のない端末でも、項目の名前が_listで終わるキーは、リストとして取り込まれる。
#[test]
fn a_key_of_an_item_that_ends_with_list_is_taken_in_as_a_list_without_its_definition() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, _, x, _) = two_devices_with_items(dir.path());
    a.ensure_defined("author_list");
    a.core
        .item_service
        .set_property(&x, "author_list", "list", "p,q")
        .unwrap();
    a.export();

    b.receive(&a, PAGE);

    let lists = b
        .core
        .properties
        .find_list_by_targets("item", &[x.as_str().to_string()])
        .unwrap();
    assert_eq!(lists[x.as_str()]["author_list"], ["p", "q"]);
}

// 項目のシステム管理キーの値は、型に合わなければ取り込まれず、項目の値が残る。アプリがその型どおりに値を読むからである。ユーザーのキーの値は、型に合わなくても、書かれた文字列のまま取り込まれる。
#[test]
fn a_value_of_a_system_key_that_does_not_fit_stays_out_and_one_of_a_user_key_comes_in() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b, _, x, _) = two_devices_with_items(dir.path());
    a.core
        .properties
        .insert_definition("effort_n", "number", false)
        .unwrap();
    a.core
        .item_service
        .set_property(&x, "zoom", "number", "1.5")
        .unwrap();
    a.core
        .item_service
        .set_property(&x, "effort_n", "number", "3")
        .unwrap();
    a.export();
    let file = a
        .file(PAGE)
        .replacen("zoom=\"1.5\"", "zoom=\"many\"", 1)
        .replacen("effort_n=\"3\"", "effort_n=\"some\"", 1);
    assert!(file.contains("zoom=\"many\"") && file.contains("effort_n=\"some\""));
    std::fs::write(
        a.root.join(PAGE),
        dated(&file, a.core.db.now_millis() + 60_000),
    )
    .unwrap();

    a.sync(PAGE);

    let scalars = a
        .core
        .properties
        .find_scalar_by_targets("item", &[x.as_str().to_string()])
        .unwrap();
    assert_eq!(scalars[x.as_str()]["zoom"], "1.5");
    assert_eq!(scalars[x.as_str()]["effort_n"], "some");
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
    a.receive(&b, PAGE);

    assert_eq!(b.bodies(&page), ["x from A", "y", "z from B"]);
    assert_eq!(a.bodies(&page), ["x from A", "y", "z from B"]);
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

// ファイルの更新時刻が中の項目の日付より古くても、相手が消した項目は消える。
#[test]
fn an_item_deleted_on_the_other_device_goes_when_the_file_is_dated_before_its_items() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let page = a.page_in("d/", "a").unwrap();
    a.add(&page, "x");
    let y = a.add(&page, "y");
    // The clock of the file system can be behind the clock that dates the items
    b.deliver(&a, PAGE);
    b.date_file(PAGE, SystemTime::now() - Duration::from_secs(60));
    b.sync(PAGE);

    a.delete(&y);
    b.receive(&a, PAGE);

    assert_eq!(b.bodies(&page), ["x"]);
    assert!(b.conflicts().is_empty());
}

// 時計の進んだ相手のファイルを取り込んだあとでも、ここで足した項目は、相手の次のファイルに無くても残る。
#[test]
fn an_item_added_here_stays_after_a_file_of_a_device_whose_clock_is_ahead() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, _, _) = two_devices_with_items(dir.path());
    let written = a.file(PAGE);
    let ahead = b.core.db.now_millis() + 3_600_000;
    let first = dated(&written.replace("\nx\n", "\nx ahead\n"), ahead);
    let second = dated(&first.replace("\ny\n", "\ny ahead\n"), ahead + 1000);
    // The cloud folder gives the file the time of this device, not of the device ahead
    std::fs::write(b.root.join(PAGE), first).unwrap();
    assert_eq!(b.sync(PAGE), Outcome::Imported);
    time_passes(&[&a, &b]);

    b.add(&page, "z from B");
    std::fs::write(b.root.join(PAGE), second).unwrap();
    b.sync(PAGE);

    assert_eq!(b.bodies(&page), ["x ahead", "y ahead", "z from B"]);
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
    b.core
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
    let items = a.core.page_service.get_page_items(&page).unwrap();
    let idx_of = |id: &Hmtid| items.iter().find(|i| i.id == *id).unwrap().idx.clone();
    let (idx_x, idx_y) = (idx_of(&x), idx_of(&y));
    let swapped = file
        .replace(&format!("idx=\"{idx_x}\""), "idx=\"swap\"")
        .replace(&format!("idx=\"{idx_y}\""), &format!("idx=\"{idx_x}\""))
        .replace("idx=\"swap\"", &format!("idx=\"{idx_y}\""));
    a.core
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

    let written = b.core.export.serialize_to_outline_markdown(
        &b.core.pages.find_by_id(&page).unwrap().unwrap(),
        &b.core.page_service.get_page_items(&page).unwrap(),
    );
    assert_eq!(modified_of(&written.unwrap(), &x), modified_of(&from_a, &x));
}

// 2台がそれぞれ足して書き出した項目は、相手のファイルを取り込むと両方に残る。
#[test]
fn items_added_and_written_on_two_devices_are_both_kept_when_one_file_comes() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, _, _) = two_devices_with_items(dir.path());
    a.add(&page, "from A");
    a.export();
    b.add(&page, "from B");
    b.export();

    b.receive(&a, PAGE);
    a.receive(&b, PAGE);

    for device in [&a, &b] {
        let mut bodies = device.bodies(&page);
        bodies.sort();
        assert_eq!(bodies, ["from A", "from B", "x", "y"]);
    }
}

// 別のページへ動かした項目は、古いファイルが届いても戻らない。
#[test]
fn an_item_moved_to_another_page_does_not_come_back_from_an_older_file() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, x, y) = two_devices_with_items(dir.path());
    let other = b.doc("other");
    a.write(&x, "x from A");
    a.export();
    b.core
        .item_service
        .move_items(std::slice::from_ref(&y), &other, &ItemPosition::Last, None)
        .unwrap();

    b.receive(&a, PAGE);

    assert_eq!(b.bodies(&page), ["x from A"]);
    assert_eq!(b.bodies(&other), ["y"]);
}

// 2台が同じ隙間に足して同じidxになると、どちらのファイルが先に届いても、項目idの順に並ぶ。
#[test]
fn items_with_the_same_index_end_in_the_order_of_their_ids_whichever_file_comes_first() {
    for larger_id_writes_first in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let (a, b, page, _, _) = two_devices_with_items(dir.path());
        let from_a = a.add(&page, "from A");
        let from_b = b.add(&page, "from B");
        assert_eq!(a.item_idx(&from_a), b.item_idx(&from_b));
        let ((smaller, smaller_body), (larger, larger_body)) = if from_a < from_b {
            ((&a, "from A"), (&b, "from B"))
        } else {
            ((&b, "from B"), (&a, "from A"))
        };
        let (writer, reader) = if larger_id_writes_first {
            (larger, smaller)
        } else {
            (smaller, larger)
        };

        writer.export();
        reader.receive(writer, PAGE);
        writer.receive(reader, PAGE);

        assert_eq!(a.bodies(&page), ["x", "y", smaller_body, larger_body]);
        assert_eq!(a.bodies(&page), b.bodies(&page));
        assert_eq!(a.indexes(&page), b.indexes(&page));
    }
}

// ページより古いファイルは、ページのタグを変えない。
#[test]
fn an_older_file_leaves_the_tags_of_the_page() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, x, _) = two_devices_with_items(dir.path());
    a.write(&x, "x from A");
    a.export();
    b.core
        .page_service
        .set_property(&page, "tags", "tags", "kept")
        .unwrap();

    b.receive(&a, PAGE);

    assert_eq!(b.core.page_service.page_tags(&page).unwrap(), ["kept"]);
}

// 手で足したページのタグは、ページの日付を変えずに書いたファイルからも取り込まれる。
#[test]
fn a_tag_added_by_hand_is_taken_in_and_gives_the_page_a_new_date() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b, page, _, _) = two_devices_with_items(dir.path());
    let before = a.file(PAGE);
    let edited = before.replacen("---\n\n", "tags:\n  - hand\n---\n\n", 1);
    assert_ne!(before, edited);
    std::fs::write(a.root.join(PAGE), edited).unwrap();

    a.sync(PAGE);

    assert_eq!(a.core.page_service.page_tags(&page).unwrap(), ["hand"]);
    assert_ne!(page_date_of(&a.file(PAGE)), page_date_of(&before));
}

// ファイルの中で同じidを持つ行は、新しい項目になる。
#[test]
fn a_line_with_an_id_used_above_it_becomes_a_new_item() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b, page, x, _) = two_devices_with_items(dir.path());
    let mut file = a.file(PAGE);
    file.push_str(&format!("\n<!-- @ {x} idx=\"a9\" -->\nsecond x\n"));
    std::fs::write(a.root.join(PAGE), file).unwrap();

    a.sync(PAGE);

    assert_eq!(a.bodies(&page), ["x", "y", "second x"]);
}

// 別のページの項目のidを持つ行は新しい項目になり、元の項目は元のページに残る。
#[test]
fn a_line_with_the_id_of_an_item_of_another_page_leaves_that_item_alone() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b, page, _, _) = two_devices_with_items(dir.path());
    let other = a.doc("other");
    let foreign = a.add(&other, "foreign");
    let mut file = a.file(PAGE);
    file.push_str(&format!("\n<!-- @ {foreign} idx=\"a9\" -->\ncopied\n"));
    std::fs::write(a.root.join(PAGE), file).unwrap();

    a.sync(PAGE);

    assert_eq!(a.bodies(&page), ["x", "y", "copied"]);
    assert_eq!(a.bodies(&other), ["foreign"]);
}

// トランスクルージョンの元の項目がもうあるページは、待たずに取り込まれ、クライアントにはページの変更が1回だけ知らされる。
#[test]
fn a_page_whose_transclusion_source_is_here_is_taken_in_without_waiting() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let showing = a.doc("b-showing");
    let source_page = a.doc("z-source");
    let source = a.add(&source_page, "source text");
    let shown = a.add(&showing, "");
    a.core
        .item_service
        .set_transclusion(&shown, &source)
        .unwrap();
    b.receive(&a, "docs/d/z-source.md");
    let since = b.told.messages().len();

    b.receive(&a, "docs/d/b-showing.md");

    let told = b.told.messages()[since..]
        .iter()
        .filter(|m| m["type"] == "pagesUpdated" && m["pageId"] == showing.to_string())
        .count();
    assert_eq!(told, 1);
    let item = b.core.item_service.get_item_by_id(&shown).unwrap();
    assert_eq!(item.transclude_item_id, Some(source));
}

// 元の項目を待っていた項目は、トランスクルージョンになるとき、元の項目の本文を受け取る。ファイルの写しが、元の項目の後の編集より古いことがあるためである。
#[test]
fn an_item_that_waited_for_its_source_takes_the_body_of_the_source() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let showing = a.doc("b-showing");
    let source_page = a.doc("z-source");
    let source = a.add(&source_page, "old text");
    let shown = a.add(&showing, "");
    a.core
        .item_service
        .set_transclusion(&shown, &source)
        .unwrap();
    b.deliver(&a, "docs/d/b-showing.md");
    a.write(&source, "new text");

    b.sync("docs/d/b-showing.md");
    b.receive(&a, "docs/d/z-source.md");

    let item = b.core.item_service.get_item_by_id(&shown).unwrap();
    assert_eq!(item.body, "new text");
}

// 元の項目を待つページのファイルは、待つ間も、トランスクルージョンになった後も、トランスクルージョンを書いたままである。待つ間に書き出すと、ファイルからつながりが消えるためである。
#[test]
fn the_file_of_a_page_that_waits_for_a_transclusion_source_keeps_the_transclusion() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let showing = a.doc("b-showing");
    let source_page = a.doc("z-source");
    let source = a.add(&source_page, "source text");
    let shown = a.add(&showing, "");
    a.core
        .item_service
        .set_transclusion(&shown, &source)
        .unwrap();
    let attr = format!(" transclude=\"{source}\"");

    // A file without its idx comments is written back, which would drop the transclusion
    b.deliver(&a, "docs/d/b-showing.md");
    let without_idx = Regex::new(r#" idx="[^"]*""#)
        .unwrap()
        .replace_all(&b.file("docs/d/b-showing.md"), "")
        .into_owned();
    std::fs::write(b.root.join("docs/d/b-showing.md"), without_idx).unwrap();
    b.sync("docs/d/b-showing.md");
    b.export();
    let while_waiting = b.file("docs/d/b-showing.md");
    b.receive(&a, "docs/d/z-source.md");
    b.export();

    assert!(while_waiting.contains(&attr), "{while_waiting}");
    let after = b.file("docs/d/b-showing.md");
    assert!(after.contains(&attr), "{after}");
}

// トランスクルージョンの元の項目より先にファイルが届いたページも取り込まれ、元の項目が入ったときにトランスクルージョンになる。空のDBに、フォルダを取り込むときの順である。
#[test]
fn a_page_whose_transclusion_source_comes_later_is_taken_in_and_transcludes_it_then() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let showing = a.doc("b-showing");
    let source_page = a.doc("z-source");
    let source = a.add(&source_page, "source text");
    let shown = a.add(&showing, "");
    a.core
        .item_service
        .set_transclusion(&shown, &source)
        .unwrap();

    let first = b.receive(&a, "docs/d/b-showing.md");
    b.receive(&a, "docs/d/z-source.md");

    assert_eq!(first, Outcome::Imported);
    let item = b.core.item_service.get_item_by_id(&shown).unwrap();
    assert_eq!(item.transclude_item_id, Some(source));
    assert_eq!(item.body, "source text");
}

// `transclude`のない古い形の行でも、本文が同じならトランスクルージョンのままである。
#[test]
fn a_line_without_transclude_stays_a_transclusion_while_its_body_is_the_same() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b, _page, x, y) = two_devices_with_items(dir.path());
    a.core.item_service.set_transclusion(&x, &y).unwrap();
    let file = a.file(PAGE);
    let attr = format!(" transclude=\"{y}\"");
    assert!(file.contains(&attr));
    std::fs::write(a.root.join(PAGE), file.replace(&attr, "")).unwrap();

    a.sync(PAGE);

    let item = a.core.item_service.get_item_by_id(&x).unwrap();
    assert_eq!(item.transclude_item_id, Some(y));
}

// `transclude`のない行で本文が変わると、トランスクルージョンではなくなる。
#[test]
fn a_line_without_transclude_and_with_a_new_body_is_no_transclusion() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b, _page, x, y) = two_devices_with_items(dir.path());
    a.core.item_service.set_transclusion(&x, &y).unwrap();
    let file = a.file(PAGE);
    let attr = format!(" transclude=\"{y}\"");
    let line = file.lines().find(|l| l.contains(&attr)).unwrap();
    let edited = file.replace(line, &format!("{}\nby hand", line.replace(&attr, "")));
    std::fs::write(a.root.join(PAGE), edited).unwrap();

    a.sync(PAGE);

    let item = a.core.item_service.get_item_by_id(&x).unwrap();
    assert_eq!(item.transclude_item_id, None);
    assert!(item.body.starts_with("by hand"), "{}", item.body);
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
    let written = b.core.export.serialize_to_outline_markdown(
        &b.core.pages.find_by_id(&page).unwrap().unwrap(),
        &b.core.page_service.get_page_items(&page).unwrap(),
    );
    let written = written.unwrap();
    // The changed item takes the date of the file, and the other item keeps its own
    assert_eq!(modified_of(&written, &x), page_date_of(&old_form));
    assert_eq!(modified_of(&written, &y), date_of_y);
}

// ページのファイルの項目には、更新日が書かれる。アトラスのファイルの項目にも、同じく書かれる。
#[test]
fn the_file_of_a_page_dates_its_items_and_so_does_the_file_of_an_atlas() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b, _, x, _) = two_devices_with_items(dir.path());
    let card = a.add(&a.atlas_id(), "[[d/a]]");

    assert!(modified_of(&a.file(PAGE), &x).is_some());
    assert!(modified_of(&a.file(ATLAS), &card).is_some());
}

// アトラスのページも、ほかのページと同じく項目ごとに取り込まれる。2台で別々のカードを変えると、両方の変更が残り、バックアップは作られない。
#[test]
fn an_atlas_is_taken_in_item_by_item_as_any_page_is() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let atlas = a.atlas_id();
    let x = a.add(&atlas, "[[d/x]]");
    let y = a.add(&atlas, "[[d/y]]");
    b.receive(&a, ATLAS);
    time_passes(&[&a, &b]);

    a.write(&x, "[[d/x-from-a]]");
    b.write(&y, "[[d/y-from-b]]");
    assert_eq!(b.receive(&a, ATLAS), Outcome::Imported);
    assert_eq!(a.receive(&b, ATLAS), Outcome::Imported);

    assert_eq!(a.bodies(&atlas), ["[[d/x-from-a]]", "[[d/y-from-b]]"]);
    assert_eq!(b.bodies(&atlas), ["[[d/x-from-a]]", "[[d/y-from-b]]"]);
    assert_eq!(a.file(ATLAS), b.file(ATLAS));
    assert!(a.conflicts().is_empty() && b.conflicts().is_empty());
}

const COPY: &str = "docs/d/a (PC's conflicted copy 2026-11-05).md";

impl Device {
    /// Puts the version into the folder as Dropbox does when it lost, and takes it in
    fn take_copy(&self, version: &str) -> Outcome {
        std::fs::write(self.root.join(COPY), version).unwrap();
        self.sync(COPY)
    }

    fn empty_items(&self, page_id: &Hmtid) -> usize {
        let items = self.core.page_service.get_page_items(page_id).unwrap();
        items.iter().filter(|i| i.body.is_empty()).count()
    }

    fn has_copy(&self) -> bool {
        self.root.join(COPY).exists()
    }
}

// 2台が書き出してファイルが行き違うと、負けた版の項目はconflicted copyから戻り、両方に残る。
#[test]
fn an_item_only_in_the_version_that_lost_comes_back_from_the_copy_on_both() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, _, _) = two_devices_with_items(dir.path());
    a.add(&page, "from A");
    let version_of_a = a.file(PAGE);
    b.add(&page, "from B");
    b.export();
    a.receive(&b, PAGE);

    assert_eq!(a.take_copy(&version_of_a), Outcome::Imported);
    b.receive(&a, PAGE);

    let mut bodies = a.bodies(&page);
    bodies.sort();
    assert_eq!(bodies, ["from A", "from B", "x", "y"]);
    assert_eq!(a.bodies(&page), b.bodies(&page));
    assert_eq!(a.indexes(&page), b.indexes(&page));
    assert!(!a.has_copy());
}

// 自分が書いたファイルと同じ内容のコピーは、そのファイルが差し替わるまで残り、そのあと合流する。
#[test]
fn a_copy_of_the_file_written_here_waits_until_that_file_is_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, _, _) = two_devices_with_items(dir.path());
    a.add(&page, "from A");
    let version_of_a = a.file(PAGE);
    b.add(&page, "from B");
    b.export();

    assert_eq!(a.take_copy(&version_of_a), Outcome::Waiting);
    assert!(a.has_copy());
    a.receive(&b, PAGE);
    assert_eq!(a.sync(COPY), Outcome::Imported);

    let mut bodies = a.bodies(&page);
    bodies.sort();
    assert_eq!(bodies, ["from A", "from B", "x", "y"]);
    assert!(!a.has_copy());
}

// コピーの新しい編集は入り、ここの新しい編集も残り、ファイルに書かれ、コピーは消える。
#[test]
fn a_copy_brings_its_later_edit_and_leaves_the_later_edit_here() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, x, y) = two_devices_with_items(dir.path());
    b.write(&y, "y from B");
    let copy = b.file(PAGE);
    a.write(&x, "x from A");

    assert_eq!(a.take_copy(&copy), Outcome::Imported);

    assert_eq!(a.bodies(&page), ["x from A", "y from B"]);
    assert!(a.file(PAGE).contains("y from B"));
    assert!(!a.has_copy());
    assert!(a.conflicts().is_empty());
}

// コピーにない項目は、書き出したあとでも消えない。
#[test]
fn an_item_not_in_the_copy_stays_even_when_it_was_written_to_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, _, y) = two_devices_with_items(dir.path());
    b.write(&y, "y from B");
    let copy = b.file(PAGE);
    a.add(&page, "here");
    a.export();
    time_passes(&[&a, &b]);

    a.take_copy(&copy);

    assert_eq!(a.bodies(&page), ["x", "y from B", "here"]);
}

// 日付が同じ項目の位置は、コピーでは変わらない。コピーの`idx`は古いことがある。
#[test]
fn a_copy_does_not_move_an_item_of_the_same_date() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b, page, x, _) = two_devices_with_items(dir.path());
    let attr = format!("idx=\"{}\"", a.item_idx(&x));
    let copy = a.file(PAGE).replace(&attr, "idx=\"z9\"");

    a.take_copy(&copy);

    assert_eq!(a.bodies(&page), ["x", "y"]);
}

// ここで消した項目は、コピーからは戻らない。
#[test]
fn an_item_deleted_here_does_not_come_back_from_a_copy() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, x, y) = two_devices_with_items(dir.path());
    b.write(&x, "x from B");
    let copy = b.file(PAGE);
    time_passes(&[&a, &b]);
    a.delete(&y);

    a.take_copy(&copy);

    assert_eq!(a.bodies(&page), ["x from B"]);
}

// 消した後に相手が編集した項目は、コピーから戻る。
#[test]
fn an_item_edited_after_it_was_deleted_here_comes_back_from_a_copy() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, _, y) = two_devices_with_items(dir.path());
    a.delete(&y);
    time_passes(&[&a, &b]);
    b.write(&y, "y from B");
    let copy = b.file(PAGE);

    a.take_copy(&copy);

    assert_eq!(a.bodies(&page), ["x", "y from B"]);
}

// コピーの編集が負けるときは、消す前に`.conflict`へ残す。
#[test]
fn an_edit_of_a_copy_that_loses_is_kept_as_a_conflict_backup() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, x, _) = two_devices_with_items(dir.path());
    b.write(&x, "x from B");
    let copy = b.file(PAGE);
    time_passes(&[&a, &b]);
    a.write(&x, "x from A");
    a.export();

    assert_eq!(a.take_copy(&copy), Outcome::Conflict);

    assert_eq!(a.bodies(&page), ["x from A", "y"]);
    assert!(!a.has_copy());
    let backups = a.conflicts();
    assert_eq!(backups.len(), 1);
    let backup = std::fs::read_to_string(a.root.join(&backups[0])).unwrap();
    assert!(backup.contains("x from B"));
}

// コピーのタグは、ページのタグに足される。
#[test]
fn the_tags_of_a_copy_are_added_to_the_tags_of_the_page() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b, page, _, _) = two_devices_with_items(dir.path());
    a.core
        .page_service
        .replace_page_tags(&page, &["here".to_string()])
        .unwrap();
    let copy = a.file(PAGE).replacen("---\n", "---\ntags: [there]\n", 1);

    a.take_copy(&copy);

    let mut tags = a.core.page_service.page_tags(&page).unwrap();
    tags.sort();
    assert_eq!(tags, ["here", "there"]);
    assert!(!a.has_copy());
    assert!(a.file(PAGE).contains("there"));
}

// 2台で別々に作ったページの版は、項目を足し、空の行を足さない。
#[test]
fn a_version_of_the_same_page_made_on_the_other_device_adds_items_and_no_empty_line() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let page = a.page_in("d/", "a").unwrap();
    a.add(&page, "from A");
    a.export();
    let empty_before = a.empty_items(&page);
    let made_on_b = b.doc("n");
    b.core
        .item_service
        .create_item(&made_on_b, &ItemPosition::Last, 1, "", None)
        .unwrap();
    b.add(&made_on_b, "from B");
    let copy = b.file("docs/d/n.md");

    a.take_copy(&copy);

    let mut bodies = a.bodies(&page);
    bodies.sort();
    assert_eq!(bodies, ["from A", "from B"]);
    assert_eq!(a.empty_items(&page), empty_before);
}

// まだファイルのないページのコピーは、ページを作らず、そのまま待つ。
#[test]
fn a_copy_of_a_page_without_a_file_here_waits() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, _, _, _) = two_devices_with_items(dir.path());
    let copy = b.file(PAGE);
    let name = "docs/d/n (PC's conflicted copy 2026-11-05).md";
    std::fs::write(a.root.join(name), copy).unwrap();

    assert_eq!(a.sync(name), Outcome::Waiting);

    assert!(a.root.join(name).exists());
    assert!(a.page_in("d/", "n").is_none());
}

// アトラスのコピーも、ほかのページのコピーと同じく合流し、消える。コピーの新しい編集は入り、ここの新しい編集も残る。
#[test]
fn a_copy_of_an_atlas_is_taken_in_as_the_copy_of_any_page_is() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let atlas = a.atlas_id();
    let x = a.add(&atlas, "[[d/x]]");
    let y = a.add(&atlas, "[[d/y]]");
    b.receive(&a, ATLAS);
    time_passes(&[&a, &b]);
    b.write(&y, "[[d/y-from-b]]");
    let copy = b.file(ATLAS);
    a.write(&x, "[[d/x-from-a]]");
    let name = ATLAS.replace(".md", " (PC's conflicted copy 2026-11-05).md");
    std::fs::write(a.root.join(&name), copy).unwrap();

    assert_eq!(a.sync(&name), Outcome::Imported);

    assert_eq!(a.bodies(&atlas), ["[[d/x-from-a]]", "[[d/y-from-b]]"]);
    assert!(!a.root.join(&name).exists());
    assert!(a.conflicts().is_empty());
}

// フォルダのスキャンは、届いていたコピーを合流する。
#[test]
fn the_scan_takes_in_a_copy_found_in_the_folder() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, page, _, y) = two_devices_with_items(dir.path());
    b.write(&y, "y from B");
    let copy = b.file(PAGE);
    std::fs::write(a.root.join(COPY), copy).unwrap();

    a.core.reconcile.scan();

    assert_eq!(a.bodies(&page), ["x", "y from B"]);
    assert!(a.file(PAGE).contains("y from B"));
    assert!(!a.has_copy());
}

// なくなったコピーは、合流も報告もしない。
#[test]
fn a_copy_already_gone_is_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b, _, _, _) = two_devices_with_items(dir.path());

    assert_eq!(a.sync(COPY), Outcome::Ignored);
}

// タスクの開始と終了を記録すると、項目の日付が進む。
#[test]
fn beginning_and_ending_a_task_move_the_date_of_the_item_forward() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b, _, x, _) = two_devices_with_items(dir.path());
    let date_of = |device: &Device| {
        device
            .core
            .item_service
            .get_item_by_id(&x)
            .unwrap()
            .modified_date
    };
    a.core.item_service.cycle_task(&x).unwrap();
    let as_task = date_of(&a);

    for _ in ["begin", "end"] {
        time_passes(&[&a]);
        let before = date_of(&a);
        let item = a.core.item_service.cycle_task(&x).unwrap();
        assert!(date_of(&a) > before);
        assert_eq!(item.modified_date, date_of(&a));
    }
    assert!(date_of(&a) > as_task);
}

// 相手がタスクを続けて進めても、手での編集と取り違えず、最後の状態が両方に届く。
#[test]
fn a_task_cycled_twice_on_the_other_device_arrives_without_a_conflict() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, _, x, _) = two_devices_with_items(dir.path());
    a.core.item_service.cycle_task(&x).unwrap();
    assert_eq!(b.receive(&a, PAGE), Outcome::Imported);

    for _ in ["begin", "end"] {
        time_passes(&[&a, &b]);
        a.core.item_service.cycle_task(&x).unwrap();
        assert_eq!(b.receive(&a, PAGE), Outcome::Imported);
        assert_eq!(b.file(PAGE), a.file(PAGE));
    }

    assert!(b.conflicts().is_empty());
    assert!(b.file(PAGE).contains(" end=\""));
}
