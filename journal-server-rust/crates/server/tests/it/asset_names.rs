//! Moving the files of `assets/` to names made of an ID, and the links with them (ADR-022).

use std::path::Path;
use std::time::SystemTime;

use chrono::{TimeZone, Utc};
use journal_server::application::asset_names;

/// A file in the folder, last changed at the given time (UTC).
fn file_changed_at(dir: &Path, name: &str, (y, mo, d, h, mi, s): (i32, u32, u32, u32, u32, u32)) {
    let time: SystemTime = Utc.with_ymd_and_hms(y, mo, d, h, mi, s).unwrap().into();
    let file = std::fs::File::create(dir.join(name)).unwrap();
    file.set_modified(time).unwrap();
}

mod plan {
    use super::*;

    // ファイルの新しい名前は、更新日時から作ったIDと、元の拡張子である。
    #[test]
    fn the_new_name_of_a_file_is_an_id_made_from_its_modified_time_and_its_extension() {
        let dir = tempfile::tempdir().unwrap();
        file_changed_at(dir.path(), "image-3.png", (2026, 2, 3, 4, 5, 6));

        let plan = asset_names::plan(dir.path()).unwrap();

        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].from, "image-3.png");
        let (id, extension) = plan[0].to.split_once('.').unwrap();
        assert_eq!(&id[..15], "20260203040506_");
        assert_eq!(id.len(), 22);
        assert_eq!(extension, "png");
    }

    // 古いファイルから順に並び、どのファイルも自分の日時を名前に持つ。
    #[test]
    fn the_files_come_oldest_first_and_each_keeps_its_own_time() {
        let dir = tempfile::tempdir().unwrap();
        file_changed_at(dir.path(), "a.png", (2026, 9, 27, 0, 0, 0));
        file_changed_at(dir.path(), "b.png", (2026, 2, 3, 4, 5, 6));

        let plan = asset_names::plan(dir.path()).unwrap();

        let times: Vec<&str> = plan.iter().map(|rename| &rename.to[..14]).collect();
        assert_eq!(times, ["20260203040506", "20260927000000"]);
        assert_eq!(plan[0].from, "b.png");
    }

    // 作成日時が更新日時より古いファイルは、作成日時を名前に持つ。
    #[test]
    fn a_file_made_before_its_modified_time_is_named_by_the_time_it_was_made() {
        let dir = tempfile::tempdir().unwrap();
        file_changed_at(dir.path(), "image.png", (2099, 1, 1, 0, 0, 0));
        let made = Utc::now().format("%Y%m%d").to_string();

        let plan = asset_names::plan(dir.path()).unwrap();

        assert_eq!(&plan[0].to[..8], made);
    }

    // すでにIDの名前を持つファイルは、対応表に入らない。
    #[test]
    fn a_file_that_already_has_an_id_for_its_name_is_left_out() {
        let dir = tempfile::tempdir().unwrap();
        file_changed_at(
            dir.path(),
            "20261110090501_QZ998PX.png",
            (2026, 11, 10, 9, 5, 1),
        );
        file_changed_at(
            dir.path(),
            "20261110090502_QZ998PY",
            (2026, 11, 10, 9, 5, 2),
        );
        file_changed_at(dir.path(), "image.png", (2026, 2, 3, 4, 5, 6));

        let plan = asset_names::plan(dir.path()).unwrap();

        let old_names: Vec<&str> = plan.iter().map(|rename| rename.from.as_str()).collect();
        assert_eq!(old_names, ["image.png"]);
    }
}
