//! Writing the markdown file of every page again, by hand, with the server stopped.

use journal_server::application::page_files;
use journal_server::infrastructure::export::path_mapper;
use regex::Regex;

use super::asset_names::Notes;

const TRIP: &str = "docs/Trip.md";

impl Notes {
    /// Leaves the file as a server left it that did not write the modified dates of the
    /// items yet: without them, and known to the database as written.
    fn file_from_an_old_server(&self, rel_path: &str) {
        let file = self.root.join(rel_path);
        let written = std::fs::read_to_string(&file).unwrap();
        let old = Regex::new(r#" modified="[^"]*""#)
            .unwrap()
            .replace_all(&written, "")
            .into_owned();
        assert_ne!(old, written);
        std::fs::write(&file, &old).unwrap();
        let state = &self.state.file_sync_state;
        let known = state.find_by_path(rel_path).unwrap().unwrap();
        let data = std::fs::metadata(&file).unwrap();
        state
            .upsert_synced(
                rel_path,
                &known.page_id,
                &path_mapper::sha256(&old),
                data.len() as i64,
                known.file_mtime,
            )
            .unwrap();
    }

    fn file(&self, rel_path: &str) -> String {
        std::fs::read_to_string(self.root.join(rel_path)).unwrap()
    }
}

// 項目の更新日時が書かれていないファイルに、項目の更新日時が書かれる。項目は変わらない。
#[test]
fn a_file_without_the_modified_dates_of_its_items_gets_them() {
    let notes = Notes::new();
    let item = notes.page("Trip", &["first day"]).remove(0);
    let with_dates = notes.file(TRIP);
    notes.file_from_an_old_server(TRIP);

    page_files::write_all(&notes.state).unwrap();

    assert_eq!(notes.file(TRIP), with_dates);
    assert_eq!(notes.item(&item.id), item);
}
