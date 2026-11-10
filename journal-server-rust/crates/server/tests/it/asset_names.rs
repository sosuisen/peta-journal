//! Moving the files of `assets/` to names made of an ID, and the links with them (ADR-022).

use std::path::Path;
use std::time::SystemTime;

use chrono::{TimeZone, Utc};
use journal_server::application::asset_names::{self, Rename};

/// A file in the folder, last changed at the given time (UTC).
fn file_changed_at(dir: &Path, name: &str, (y, mo, d, h, mi, s): (i32, u32, u32, u32, u32, u32)) {
    let time: SystemTime = Utc.with_ymd_and_hms(y, mo, d, h, mi, s).unwrap().into();
    let file = std::fs::File::create(dir.join(name)).unwrap();
    file.set_modified(time).unwrap();
}

fn new_names(plan: &[Rename]) -> Vec<&str> {
    plan.iter().map(|rename| rename.to.as_str()).collect()
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
}
