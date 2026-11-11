//! The entrance for the Android shell, called here from Rust as Kotlin calls it.

use crate::{item_html, PetaCore, PetaShell};
use crux_core::bridge::{BincodeFfiFormat, FfiFormat, Request};
use peta_core::Event;
use peta_shell_native::ShellEffectFfi;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// A shell that keeps what the core asks of it.
#[derive(Default)]
struct Kept {
    requests: Mutex<Vec<Request<ShellEffectFfi>>>,
}

impl PetaShell for Kept {
    fn process_effects(&self, bytes: Vec<u8>) {
        let sent: Vec<Request<ShellEffectFfi>> = BincodeFfiFormat::deserialize(&bytes).unwrap();
        self.requests.lock().unwrap().extend(sent);
    }
}

/// The folders of an app: the data of the journal, and the Markdown files from Dropbox.
struct App {
    dir: TempDir,
}

impl App {
    fn new() -> App {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("markdown/docs")).unwrap();
        App { dir }
    }

    fn markdown(&self) -> &Path {
        self.dir.path()
    }

    /// Puts a doc page into the Markdown folder, as a download does.
    fn download(&self, name: &str, text: &str) {
        let file = self.markdown().join("markdown/docs").join(name);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    }

    /// Takes a doc page out of the Markdown folder, as a sync does when Dropbox lost it.
    fn remove(&self, name: &str) {
        std::fs::remove_file(self.markdown().join("markdown/docs").join(name)).unwrap();
    }

    fn open(&self, shell: Arc<Kept>) -> PetaCore {
        let data = self.dir.path().join("data");
        let markdown = self.dir.path().join("markdown");
        PetaCore::new(path(&data), path(&markdown), shell).unwrap()
    }
}

fn path(path: &Path) -> String {
    path.to_str().unwrap().to_string()
}

fn event(event: &Event) -> Vec<u8> {
    let mut bytes = Vec::new();
    BincodeFfiFormat::serialize(&mut bytes, event).unwrap();
    bytes
}

/// The view model as text, to look for a word in it.
fn shown(core: &PetaCore) -> String {
    String::from_utf8_lossy(&core.view()).into_owned()
}

/// The doc pages the journal holds, each as its folder and its name, without the root page.
fn page_names(core: &PetaCore) -> Vec<String> {
    let pages = core.core.pages.find_all().unwrap();
    let mut names: Vec<String> = pages
        .iter()
        .map(|page| format!("{}{}", page.dir_path, page.name))
        .filter(|name| name != "index")
        .collect();
    names.sort();
    names
}

/// Waits for what the threads of the shell do. A test that waits in vain fails here.
fn wait_until(what: &str, done: impl Fn() -> bool) {
    let started = Instant::now();
    while !done() {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "waited in vain until {what}"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

// ダウンロードしたMarkdownを取り込むと、検索で見つかる。取り込んだファイルの数が返る。
#[test]
fn a_downloaded_file_is_found_by_the_search_after_a_sync() {
    let app = App::new();
    app.download("notes.md", "- 今日は東京都で会議がありました。\n");
    let core = app.open(Arc::default());

    assert_eq!(core.sync(), 1);

    core.update(&event(&Event::SearchQueryChanged("会議".into())))
        .unwrap();
    wait_until("the search shows the page", || {
        shown(&core).contains("notes")
    });
}

// 変わっていないフォルダーをもう一度取り込んでも、何も取り込まない。
#[test]
fn a_second_sync_of_the_same_folder_takes_in_nothing() {
    let app = App::new();
    app.download("notes.md", "- text\n");
    let core = app.open(Arc::default());
    core.sync();

    assert_eq!(core.sync(), 0);
}

// アプリを開き直しても、取り込んだページは残っている。
#[test]
fn the_pages_are_still_there_when_the_app_opens_again() {
    let app = App::new();
    app.download("notes.md", "- 会議\n");
    app.open(Arc::default()).sync();

    let core = app.open(Arc::default());

    assert_eq!(core.sync(), 0);
    core.update(&event(&Event::SearchQueryChanged("会議".into())))
        .unwrap();
    wait_until("the search shows the page", || {
        shown(&core).contains("notes")
    });
}

// コアを閉じたあとは、残ったスレッドがジャーナルに頼んでも、エラーが返る。
#[test]
fn an_operation_after_the_core_has_closed_is_an_error() {
    let app = App::new();
    let core = app.open(Arc::default());
    let door = core.door.clone();

    drop(core);

    assert!(door.call(peta_core::effects::ApiOp::GetConfig).is_err());
}

// URLでページを開くと、シェルに依頼が届き、ビューモデルにそのページが入る。何も指さないURLは、エラーの文になる。
#[test]
fn a_page_is_opened_by_its_url() {
    let app = App::new();
    app.download("notes.md", "- an apple\n");
    let shell = Arc::new(Kept::default());
    let core = app.open(shell.clone());
    core.sync();

    core.open("/pages/notes".into()).unwrap();

    wait_until("the page shows", || shown(&core).contains("an apple"));
    assert!(!shell.requests.lock().unwrap().is_empty());
    assert!(core.open(String::new()).is_err());
}

// 今日のジャーナルを開くと、アプリの時計の日付のページが入る。
#[test]
fn the_journal_of_today_opens_with_the_date_of_the_clock() {
    let app = App::new();
    let core = app.open(Arc::default());

    core.open("/today".into()).unwrap();

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    wait_until("the journal of today shows", || {
        shown(&core).contains(&today)
    });
}

// 読めないバイト列は、イベントとしても答えとしても、エラーの文として返る。
#[test]
fn bytes_that_cannot_be_read_come_back_as_an_error() {
    let app = App::new();
    let core = app.open(Arc::default());

    assert!(core.update(&[0xff; 3]).is_err());
    assert!(core.resolve(9_999, &[]).is_err());
}

// データのフォルダーを作れないときは、開けずにエラーの文が返る。
#[test]
fn the_app_does_not_open_when_its_data_folder_cannot_be_made() {
    let app = App::new();
    let file = app.dir.path().join("file");
    std::fs::write(&file, "").unwrap();

    let opened = PetaCore::new(
        path(&file.join("data")),
        path(app.markdown()),
        Arc::<Kept>::default(),
    );

    assert!(opened.is_err());
}

// Dropboxで消えたファイルのページは、次の取り込みで、尋ねられずにアプリからも消える。
#[test]
fn the_page_of_a_file_that_has_gone_is_deleted_by_the_next_sync() {
    let app = App::new();
    app.download(
        "notes.md", "- text
",
    );
    app.download(
        "other.md", "- text
",
    );
    let core = app.open(Arc::default());
    core.sync();
    app.remove("notes.md");

    assert_eq!(core.sync(), 0);

    assert_eq!(page_names(&core), ["other"]);
}

// フォルダーごと消えたときは、子のページもフォルダーのページも、1回の取り込みで消える。
#[test]
fn the_pages_of_a_folder_that_has_gone_are_deleted_by_one_sync() {
    let app = App::new();
    app.download(
        "other.md", "- text
",
    );
    app.download(
        "work/index.md",
        "- text
",
    );
    app.download(
        "work/plan.md",
        "- text
",
    );
    let core = app.open(Arc::default());
    core.sync();
    app.remove("work/index.md");
    app.remove("work/plan.md");

    core.sync();

    assert_eq!(page_names(&core), ["other"]);
}

// 子のページが残っているフォルダーのページは、ファイルが消えても残る。取り込みは終わる。
#[test]
fn the_page_of_a_folder_stays_while_a_page_under_it_is_left() {
    let app = App::new();
    app.download(
        "work/index.md",
        "- text
",
    );
    app.download(
        "work/plan.md",
        "- text
",
    );
    let core = app.open(Arc::default());
    core.sync();
    app.remove("work/index.md");

    core.sync();

    assert_eq!(page_names(&core), ["work/index", "work/plan"]);
}

// Markdownのフォルダーの設定が変わっても、アプリは何も見張らないので、何も起きない。
#[test]
fn a_new_markdown_folder_starts_no_watcher() {
    let app = App::new();
    let core = app.open(Arc::default());
    let other = app.dir.path().join("other");
    std::fs::create_dir_all(&other).unwrap();
    let input = serde_json::from_value(serde_json::json!({ "markdownDir": path(&other) })).unwrap();

    core.core.journal.update_config(input);

    assert_eq!(core.core.config.markdown_dir(), path(&other));
}

// 項目のASTから、HTMLを取れる。
#[test]
fn the_html_of_an_item_comes_from_its_ast() {
    let ast = r#"{"type":"document","children":[{"type":"paragraph","children":[{"type":"text","value":"text"}]}]}"#;

    assert_eq!(item_html(ast.into()).unwrap(), "<p>text</p>\n");
}
