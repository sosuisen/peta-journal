//! Writing the markdown file of every page again, by hand, with the server stopped.

use journal_core::infrastructure::export::path_mapper;
use journal_server::application::page_files;
use regex::Regex;

use super::asset_names::{settings_under, Notes};

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
                known.last_synced,
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

// まだ取り込まれていないファイルの編集は、失われない。先に取り込まれる。
#[test]
fn an_edit_of_a_file_that_was_not_taken_in_yet_is_kept() {
    let notes = Notes::new();
    let item = notes.page("Trip", &["first day"]).remove(0);
    let edited = notes.file(TRIP).replace("first day", "first day in Kyoto");
    std::fs::write(notes.root.join(TRIP), edited).unwrap();

    page_files::write_all(&notes.state).unwrap();

    assert_eq!(notes.item(&item.id).body, "first day in Kyoto");
    assert!(notes.file(TRIP).contains("first day in Kyoto"));
}

// コマンドは、止まっているサーバーのノートのファイルを書き直し、書いたページの数を伝える。
#[test]
fn the_command_writes_the_files_of_a_stopped_server_and_tells_how_many_pages_it_wrote() {
    let notes = Notes::new();
    notes.page("Trip", &["first day"]);
    notes.page("Home", &["back"]);
    let with_dates = notes.file(TRIP);
    notes.file_from_an_old_server(TRIP);
    let notes = notes.stop();

    let told = page_files::run(settings_under(notes.dir.path())).unwrap();

    assert_eq!(notes.file(TRIP), with_dates);
    // The third page is the atlas of the docs, which the server makes
    assert_eq!(told, "1 of 3 page files written");
}

// 待ち行列のページがいくつあっても、1回の呼び出しですべてのファイルが書かれる。
#[test]
fn every_page_that_waits_is_written_by_one_call() {
    let notes = Notes::new();
    let names: Vec<String> = (0..25).map(|n| format!("Page{n}")).collect();
    for name in &names {
        let pages = &notes.state.page_service;
        pages.get_or_create_doc(Some(name), None).unwrap();
    }

    notes.state.export.process_whole_queue();

    let missing: Vec<&String> = names
        .iter()
        .filter(|name| !notes.root.join(format!("docs/{name}.md")).is_file())
        .collect();
    assert!(missing.is_empty(), "not written: {missing:?}");
}
