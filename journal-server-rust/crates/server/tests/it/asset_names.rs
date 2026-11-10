//! Moving the files of `assets/` to names made of an ID, and the links with them (ADR-022).

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{TimeZone, Utc};
use hmtid::Hmtid;
use journal_server::application::asset_names::{self, Rename};
use journal_server::config::{AssetNamesStep, Settings};
use journal_server::domain::model::{Item, ItemPosition, Page};
use journal_server::state::{AppState, BuildOptions, SharedState};
use serde_json::json;

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

fn settings_under(dir: &Path) -> Settings {
    Settings {
        // A scan that finds missing files opens the browser, unless it did within the
        // cooldown; one longer than the time since 1970 keeps it shut
        import_delete_prompt_cooldown_secs: 10_000_000_000,
        ..Settings::under(dir, 0)
    }
}

/// A server that is not running, with its markdown folder: what the command works on.
struct Notes {
    state: SharedState,
    root: PathBuf,
    _dir: tempfile::TempDir,
}

impl Notes {
    fn new() -> Notes {
        let dir = tempfile::Builder::new()
            .prefix("peta-journal-asset-names-")
            .tempdir()
            .unwrap();
        let settings = settings_under(dir.path());
        let root = dir.path().join("markdown");
        let config_dir = dir.path().join("config");
        std::fs::create_dir_all(root.join("assets")).unwrap();
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(
            config_dir.join("config.json"),
            json!({ "markdownDir": root.to_string_lossy() }).to_string(),
        )
        .unwrap();
        let state = AppState::build(settings, BuildOptions { interactive: false }).unwrap();
        Notes {
            state,
            root,
            _dir: dir,
        }
    }

    /// A file in `assets/` with the given content.
    fn asset(&self, name: &str, content: &str) {
        std::fs::write(self.asset_path(name), content).unwrap();
    }

    fn asset_path(&self, name: &str) -> PathBuf {
        self.root.join("assets").join(name)
    }

    /// A doc page with one item per body, written to its file as the running server does.
    fn page(&self, name: &str, bodies: &[&str]) -> Vec<Item> {
        let page = self
            .state
            .page_service
            .get_or_create_doc(Some(name), None)
            .unwrap()
            .page;
        let items = bodies
            .iter()
            .map(|body| {
                self.state
                    .item_service
                    .create_item(&page.id, &ItemPosition::Last, 1, body, None)
                    .unwrap()
            })
            .collect();
        self.state.export.process_export_queue();
        items
    }

    fn item(&self, id: &Hmtid) -> Item {
        self.state.item_service.get_item_by_id(id).unwrap()
    }

    fn page_of(&self, item: &Item) -> Page {
        self.state.page_service.require_page(&item.page_id).unwrap()
    }

    fn rename(&self, from: &str, to: &str) {
        let renames = [Rename {
            from: from.to_string(),
            to: to.to_string(),
        }];
        asset_names::apply(&self.state, &renames).unwrap();
    }
}

/// The notes of a server that was stopped: what the command finds when it is run by hand.
struct Stopped {
    root: PathBuf,
    dir: tempfile::TempDir,
}

impl Notes {
    fn stop(self) -> Stopped {
        self.state.shutdown();
        Stopped {
            root: self.root,
            dir: self._dir,
        }
    }
}

impl Stopped {
    fn table(&self) -> PathBuf {
        self.dir.path().join("asset-names.json")
    }

    fn run(&self, step: AssetNamesStep) -> String {
        asset_names::run(settings_under(self.dir.path()), step, &self.table()).unwrap()
    }

    fn file(&self, rel_path: &str) -> String {
        std::fs::read_to_string(self.root.join(rel_path)).unwrap()
    }

    fn assets(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(self.root.join("assets"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

const NEW_NAME: &str = "20260203040506_ABCDEFG.png";

mod apply {
    use super::*;

    // ファイルは、中身はそのままで、新しい名前に変わる。
    #[test]
    fn the_file_moves_to_its_new_name_with_its_content() {
        let notes = Notes::new();
        notes.asset("image.png", "the picture");

        notes.rename("image.png", NEW_NAME);

        assert!(!notes.asset_path("image.png").exists());
        let moved = std::fs::read_to_string(notes.asset_path(NEW_NAME)).unwrap();
        assert_eq!(moved, "the picture");
    }

    // 項目の本文のリンクは、新しい名前を指す。項目とページの更新日時は変わらない。
    #[test]
    fn the_link_in_an_item_goes_to_the_new_name_and_the_modified_dates_stay() {
        let notes = Notes::new();
        notes.asset("image.png", "the picture");
        let before = notes
            .page("Trip", &["![pic](../assets/image.png)"])
            .remove(0);
        let page_before = notes.page_of(&before);

        notes.rename("image.png", NEW_NAME);

        let after = notes.item(&before.id);
        assert_eq!(after.body, format!("![pic](../assets/{NEW_NAME})"));
        assert!(after.ast.contains(NEW_NAME));
        assert!(!after.ast.contains("image.png"));
        assert_eq!(after.modified_date, before.modified_date);
        assert_eq!(
            notes.page_of(&after).modified_date,
            page_before.modified_date
        );
    }

    // ページのMarkdownファイルのリンクも、新しい名前を指す。そのあとフォルダを調べても、
    // 取り込むものはなく、項目は変わらない。
    #[test]
    fn the_file_of_the_page_has_the_new_name_and_a_later_scan_takes_nothing_in() {
        let notes = Notes::new();
        notes.asset("image.png", "the picture");
        let item = notes
            .page("Trip", &["![pic](../assets/image.png)"])
            .remove(0);

        notes.rename("image.png", NEW_NAME);

        let text = std::fs::read_to_string(notes.root.join("docs/Trip.md")).unwrap();
        assert!(text.contains(&format!("![pic](../assets/{NEW_NAME})")));
        assert!(!text.contains("image.png"));
        let renamed = notes.item(&item.id);
        assert_eq!(notes.state.reconcile.scan().imported, 0);
        let scanned = notes.item(&item.id);
        assert_eq!(scanned.body, renamed.body);
        assert_eq!(scanned.modified_date, renamed.modified_date);
    }

    // まだ取り込まれていないMarkdownファイルの編集は、失われない。先に取り込まれる。
    #[test]
    fn an_edit_of_the_file_that_was_not_taken_in_yet_is_kept() {
        let notes = Notes::new();
        notes.asset("image.png", "the picture");
        let items = notes.page("Trip", &["![pic](../assets/image.png)", "first day"]);
        let file = notes.root.join("docs/Trip.md");
        let edited = std::fs::read_to_string(&file)
            .unwrap()
            .replace("first day", "first day in Kyoto");
        std::fs::write(&file, edited).unwrap();

        notes.rename("image.png", NEW_NAME);

        assert_eq!(notes.item(&items[1].id).body, "first day in Kyoto");
        assert_eq!(
            notes.item(&items[0].id).body,
            format!("![pic](../assets/{NEW_NAME})")
        );
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("first day in Kyoto"));
        assert!(text.contains(NEW_NAME));
    }

    // 検索の結果に出る本文も、新しい名前を指す。
    #[test]
    fn the_body_a_search_shows_has_the_new_name() {
        let notes = Notes::new();
        notes.asset("image.png", "the picture");
        notes.page("Trip", &["Kyoto ![pic](../assets/image.png)"]);

        notes.rename("image.png", NEW_NAME);

        let found = notes.state.search.search_full_text("Kyoto", 10);
        assert_eq!(found.len(), 1);
        assert!(found[0].snippet.contains(NEW_NAME));
        assert!(!found[0].snippet.contains("image.png"));
    }

    // ファイルがすでに新しい名前になっていても、リンクは書き換わる。途中で止まった移行を
    // やり直すときと、フォルダの同期が先にファイルを運んできた別の端末で起きる。
    #[test]
    fn the_links_change_when_the_file_is_already_under_its_new_name() {
        let notes = Notes::new();
        notes.asset(NEW_NAME, "the picture");
        let item = notes
            .page("Trip", &["![pic](../assets/image.png)"])
            .remove(0);

        notes.rename("image.png", NEW_NAME);

        assert_eq!(
            notes.item(&item.id).body,
            format!("![pic](../assets/{NEW_NAME})")
        );
        let kept = std::fs::read_to_string(notes.asset_path(NEW_NAME)).unwrap();
        assert_eq!(kept, "the picture");
    }
}

mod command {
    use super::*;

    // 「plan」は、対応表をファイルに書くだけで、ノートは変えない。
    #[test]
    fn the_plan_step_writes_the_table_to_the_file_and_changes_nothing() {
        let notes = Notes::new();
        notes.asset("image.png", "the picture");
        notes.page("Trip", &["![pic](../assets/image.png)"]);
        let notes = notes.stop();
        let page_before = notes.file("docs/Trip.md");

        notes.run(AssetNamesStep::Plan);

        let table: Vec<Rename> =
            serde_json::from_str(&std::fs::read_to_string(notes.table()).unwrap()).unwrap();
        assert_eq!(table.len(), 1);
        assert_eq!(table[0].from, "image.png");
        assert!(table[0].to.ends_with(".png"));
        assert_eq!(notes.assets(), ["image.png"]);
        assert_eq!(notes.file("docs/Trip.md"), page_before);
    }

    // 「apply」は、ファイルの対応表のとおりに、ファイルとリンクを変える。
    #[test]
    fn the_apply_step_moves_the_files_and_the_links_as_the_table_in_the_file_says() {
        let notes = Notes::new();
        notes.asset("image.png", "the picture");
        notes.page("Trip", &["![pic](../assets/image.png)"]);
        let notes = notes.stop();
        notes.run(AssetNamesStep::Plan);
        let table: Vec<Rename> =
            serde_json::from_str(&std::fs::read_to_string(notes.table()).unwrap()).unwrap();

        notes.run(AssetNamesStep::Apply);

        let new_name = &table[0].to;
        assert_eq!(notes.assets(), [new_name.as_str()]);
        assert!(notes
            .file("docs/Trip.md")
            .contains(&format!("![pic](../assets/{new_name})")));
    }
}
