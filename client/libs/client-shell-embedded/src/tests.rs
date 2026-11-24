//! The entrance for the Android shell, called here from Rust as Kotlin calls it.

use crate::{PetaCore, PetaScreen, PetaShell};
use client_core::typing::Typing;
use client_core::{EditPlace, Event};
use client_native_bridge::ShellEffectFfi;
use crux_core::bridge::{BincodeFfiFormat, FfiFormat, Request};
use journal_core::domain::model::ItemPosition;
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

    /// Puts a journal into the Markdown folder, as a download does.
    fn download_journal(&self, name: &str, text: &str) {
        let folder = self.markdown().join("markdown/journals");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join(format!("{name}.md")), text).unwrap();
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
    String::from_utf8_lossy(&core.shell.view()).into_owned()
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

    assert!(door.call(client_core::effects::ApiOp::GetConfig).is_err());
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

// 入口から、画面の差分をbytesで読める。開いたページの行が、最初の差分に入る（ADR 076）。
#[test]
fn the_changes_of_the_screen_are_read_as_bytes() {
    let app = App::new();
    app.download("notes.md", "- an apple\n");
    let core = app.open(Arc::default());
    core.sync();
    core.open("/pages/notes".into()).unwrap();
    wait_until("the page shows", || shown(&core).contains("an apple"));

    let changes = core.changes();

    assert!(String::from_utf8_lossy(&changes).contains("an apple"));
}

// アプリが裏に回るときにコアを閉じると、保存を待っていた本文が、閉じる前にジャーナルに保存される（#766）。
#[test]
fn closing_saves_the_body_still_waiting_to_be_saved() {
    let app = App::new();
    app.download("notes.md", "- an apple\n");
    let core = app.open(Arc::default());
    core.sync();
    core.open("/pages/notes".into()).unwrap();
    wait_until("the page shows", || shown(&core).contains("an apple"));
    let page = core
        .core
        .page_service
        .find_doc("notes", "")
        .unwrap()
        .unwrap();
    let item_id = core.core.items.find_by_page_id(&page.id).unwrap()[0]
        .id
        .to_string();
    core.update(&event(&Event::EditRequested {
        item_id: item_id.clone(),
        place: EditPlace::Unplaced,
    }))
    .unwrap();
    let typing = Typing::Typed {
        item_id,
        body: "a pear".into(),
        cursor: 6,
    };
    let mut bytes = Vec::new();
    BincodeFfiFormat::serialize(&mut bytes, &typing).unwrap();
    core.typed(&bytes).unwrap();

    core.leave();

    let items = core.core.items.find_by_page_id(&page.id).unwrap();
    assert_eq!(items[0].body, "a pear");
}

// シェルがパスで貼った画像は、本文フォルダの`assets/`に写され、項目にそのリンクが入る。一時ファイルは消える（ADR 071）。
#[test]
fn an_image_pasted_by_its_path_goes_into_the_assets_and_its_link_into_the_item() {
    let app = App::new();
    app.download("notes.md", "- an apple\n");
    let core = app.open(Arc::default());
    core.sync();
    core.open("/pages/notes".into()).unwrap();
    wait_until("the page shows", || shown(&core).contains("an apple"));
    let page = core
        .core
        .page_service
        .find_doc("notes", "")
        .unwrap()
        .unwrap();
    let item_id = core.core.items.find_by_page_id(&page.id).unwrap()[0]
        .id
        .to_string();
    let pasted = app.dir.path().join("pasted.png");
    std::fs::write(&pasted, b"drawing").unwrap();

    core.paste_image(item_id, path(&pasted), 0);

    let body = || {
        core.core.items.find_by_page_id(&page.id).unwrap()[0]
            .body
            .clone()
    };
    wait_until("the link is in the item", || body().contains("../assets/"));
    let saved = body()
        .split("../assets/")
        .nth(1)
        .unwrap()
        .split(')')
        .next()
        .unwrap()
        .to_string();
    let copy = app.markdown().join("markdown/assets").join(saved);
    assert_eq!(std::fs::read(copy).unwrap(), b"drawing");
    assert!(!pasted.exists());
}

// 今日のジャーナルを開くと、アプリの時計の日付のページが入る。
#[test]
fn the_journal_of_today_opens_with_the_date_of_the_clock() {
    let app = App::new();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    app.download_journal(&today, "the weather was fine\n");
    let core = app.open(Arc::default());
    core.sync();

    core.open("/today".into()).unwrap();

    wait_until("the journal of today shows", || {
        shown(&core).contains("the weather was fine")
    });
}

// フォルダーの名前のURLは、そのフォルダーのindexページを開く。
#[test]
fn the_url_of_a_folder_opens_its_index_page() {
    let app = App::new();
    app.download("work/plan.md", "the plan\n");
    let core = app.open(Arc::default());
    core.sync();

    core.open("/pages/work".into()).unwrap();

    wait_until("the index page shows", || shown(&core).contains("index"));
}

// まだないページのURLは、開かずにエラーの文になる。ページもジャーナルも作られない。
#[test]
fn a_url_of_a_page_that_is_not_there_opens_nothing_and_makes_no_page() {
    let app = App::new();
    app.download("notes.md", "an apple\n");
    let core = app.open(Arc::default());
    core.sync();
    let before = core.core.pages.find_all().unwrap().len();

    assert!(core.open("/pages/ghost".into()).is_err());
    assert!(core.open("/pages/nowhere/ghost#item".into()).is_err());
    assert!(core.open("/pages/2001-02-03".into()).is_err());
    assert!(core.open("/today".into()).is_err());

    assert_eq!(core.core.pages.find_all().unwrap().len(), before);
}

// 開けなかったときのエラーは、そこにないページの名前である。名前には、URLのフラグメントが付かない。
#[test]
fn the_error_of_a_url_that_opens_nothing_is_the_name_of_the_page() {
    let app = App::new();
    let core = app.open(Arc::default());

    assert_eq!(core.open("/pages/ghost".into()), Err("ghost".into()));
    assert_eq!(
        core.open("/pages/nowhere/ghost#item".into()),
        Err("nowhere/ghost".into())
    );
    assert_eq!(core.open("/today".into()), Err("/today".into()));
}

// 開いた画面のURLは、アプリを開き直したあとも、最後の画面として残る。開けなかったURLは残らない。
#[test]
fn the_url_last_opened_stays_when_the_app_opens_again() {
    let app = App::new();
    app.download("notes.md", "an apple\n");
    let core = app.open(Arc::default());
    assert_eq!(core.last_screen(), None);
    core.sync();

    core.open("/pages/notes".into()).unwrap();
    assert!(core.open("/pages/ghost".into()).is_err());
    drop(core);

    let core = app.open(Arc::default());
    assert_eq!(core.last_screen(), Some("/pages/notes".into()));
    core.open_or_make("/pages/work/plan".into()).unwrap();
    assert_eq!(core.last_screen(), Some("/pages/work/plan".into()));
}

// 作るつもりで開くと、まだないページが作られて開く。今日のジャーナルも同じである。
#[test]
fn a_page_that_is_not_there_is_made_when_it_is_opened_to_be_made() {
    let app = App::new();
    let core = app.open(Arc::default());
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    core.open_or_make("/pages/work/plan".into()).unwrap();
    wait_until("the page is made", || {
        page_names(&core).contains(&"work/plan".to_string())
    });
    core.open_or_make("/today".into()).unwrap();

    wait_until("the journal of today is made", || {
        let journals = &core.core.pages;
        let made = journals.find_by_type_dir_path_file_name("journal", "", &today);
        matches!(made, Ok(Some(_)))
    });
    assert!(core.open_or_make(String::new()).is_err());
}

// 設定の画面のURLは、ページがなくても開く。
#[test]
fn the_url_of_the_settings_opens_without_a_page() {
    let app = App::new();
    let core = app.open(Arc::default());

    assert!(core.open("/settings".into()).is_ok());
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

// 同期で消えたページには戻らず、そのページもファイルも作り直さない。その前のページが開く（#763）。
#[test]
fn going_back_does_not_make_again_a_page_that_sync_deleted() {
    let app = App::new();
    app.download("first.md", "- first text\n");
    app.download("gone.md", "- gone text\n");
    app.download("last.md", "- last text\n");
    let core = app.open(Arc::default());
    core.sync();
    for url in ["/pages/first", "/pages/gone", "/pages/last"] {
        core.open(url.into()).unwrap();
    }
    wait_until("the last page shows", || shown(&core).contains("last text"));
    app.remove("gone.md");
    core.sync();

    core.update(&event(&Event::Back {
        today: "2026-11-20".into(),
    }))
    .unwrap();

    wait_until("the first page shows", || {
        shown(&core).contains("first text")
    });
    core.sync();
    assert_eq!(page_names(&core), ["first", "last"]);
    assert!(!app.markdown().join("markdown/docs/gone.md").exists());
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

// Dropboxで消えたファイルのページでも、アプリで変えてまだ書いていないものは消えない。ファイルが書き直される。
#[test]
fn a_page_that_changed_in_the_app_stays_when_its_file_has_gone() {
    let app = App::new();
    app.download("notes.md", "- text\n");
    app.download("other.md", "- text\n");
    let core = app.open(Arc::default());
    core.sync();
    let notes = core.core.page_service.find_doc("notes", "").unwrap();
    let items = &core.core.item_service;
    items
        .create_item(
            &notes.unwrap().id,
            &ItemPosition::Last,
            1,
            "from the phone",
            None,
        )
        .unwrap();
    app.remove("notes.md");
    app.remove("other.md");

    core.sync();

    assert_eq!(page_names(&core), ["notes"]);
    let written = std::fs::read_to_string(app.markdown().join("markdown/docs/notes.md"));
    assert!(written.unwrap().contains("from the phone"));
}

// アプリで作ったページは、次の取り込みでMarkdownのフォルダーに書かれる。
#[test]
fn a_page_made_in_the_app_is_written_to_the_folder_by_the_next_sync() {
    let app = App::new();
    let core = app.open(Arc::default());
    let journal = &core.core.journal;
    journal.get_or_create_doc(Some("notes"), None).unwrap();

    core.sync();

    assert!(app.markdown().join("markdown/docs/notes.md").is_file());
}

// 書くページがいくつあっても、1回の取り込みですべて書かれる。
#[test]
fn every_page_that_waits_is_written_by_one_sync() {
    let app = App::new();
    let core = app.open(Arc::default());
    let journal = &core.core.journal;
    let names: Vec<String> = (0..25).map(|n| format!("page{n}")).collect();
    for name in &names {
        journal.get_or_create_doc(Some(name), None).unwrap();
    }

    core.sync();

    let docs = app.markdown().join("markdown/docs");
    let missing: Vec<&String> = names
        .iter()
        .filter(|name| !docs.join(format!("{name}.md")).is_file())
        .collect();
    assert!(missing.is_empty(), "not written: {missing:?}");
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

// 同期で届いた新しいページは、開いている画面の最近のページとディレクトリー一覧に入る。コアは、取り込みの知らせで2つを取り直す。
#[test]
fn a_page_that_a_sync_brought_comes_into_the_lists_of_the_screen() {
    let app = App::new();
    app.download("notes.md", "- an apple\n");
    let core = app.open(Arc::default());
    core.sync();
    core.open("/pages/notes".into()).unwrap();
    wait_until("the page shows", || shown(&core).contains("an apple"));
    assert!(!shown(&core).contains("fresh"));

    app.download("fresh.md", "- a pear\n");
    core.sync();

    // Each list names the page 3 times: the name of the page, its label and its path. The view
    // model cannot be read back from its bytes here, so the names are counted
    wait_until("both lists show the new page", || {
        shown(&core).matches("fresh").count() >= 6
    });
}

/// True when the journal of today is in the journal.
fn today_is_made(core: &PetaCore) -> bool {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let made = core
        .core
        .pages
        .find_by_type_dir_path_file_name("journal", "", &today);
    matches!(made, Ok(Some(_)))
}

/// The screen tells nothing: a page opened.
fn opened() -> PetaScreen {
    PetaScreen::default()
}

// ページのURLをコアが求めると、そのページが開き、画面は何も伝えない。リンクに触れたときである（ADR 083）。
#[test]
fn a_url_of_a_page_that_the_core_asks_for_opens_it_and_the_screen_tells_nothing() {
    let app = App::new();
    app.download("notes.md", "an apple\n");
    let core = app.open(Arc::default());
    core.sync();

    assert_eq!(core.navigated("/pages/notes".into()), opened());

    wait_until("the page shows", || shown(&core).contains("an apple"));
}

// 今日のジャーナルを頼むと、なければ作られて開く。
#[test]
fn today_makes_the_journal_of_today_and_opens_it() {
    let app = App::new();
    let core = app.open(Arc::default());

    assert_eq!(core.today(), opened());

    wait_until("the journal of today is made", || today_is_made(&core));
}

// コアが求めたURLは、ページがなければ作って開く。検索で「作る」を選んだときである。
#[test]
fn a_url_that_the_core_asks_for_makes_its_page() {
    let app = App::new();
    let core = app.open(Arc::default());

    assert_eq!(core.navigated("/pages/fresh".into()), opened());

    wait_until("the page is made", || {
        page_names(&core).contains(&"fresh".to_string())
    });
}

// コアが求めた符号化したURLのページがなければ、復号した名前のページができる。ないページへのリンクに触れたときである（ADR 083）。
#[test]
fn a_page_of_an_encoded_url_is_made_by_its_decoded_name() {
    let app = App::new();
    let core = app.open(Arc::default());

    assert_eq!(
        core.navigated("/pages/C%23%20notes#20260426035700_QZ998PX".into()),
        opened()
    );

    wait_until("the page is made", || {
        page_names(&core).contains(&"C# notes".to_string())
    });
}

// アプリを開くと、前に見ていたページが開く。今日のジャーナルは作られない。
#[test]
fn the_app_opens_at_the_page_it_showed_before() {
    let app = App::new();
    app.download("notes.md", "an apple\n");
    let core = app.open(Arc::default());
    core.sync();
    core.navigated("/pages/notes".into());
    drop(core);
    let core = app.open(Arc::default());

    assert_eq!(core.restore(true), opened());

    wait_until("the page shows", || shown(&core).contains("an apple"));
    assert!(!today_is_made(&core));
}

// 前に見ていたページがなければ、今日のジャーナルが作られて開く。
#[test]
fn the_app_opens_at_today_when_it_showed_no_page_before() {
    let app = App::new();
    let core = app.open(Arc::default());

    assert_eq!(core.restore(true), opened());

    wait_until("the journal of today is made", || today_is_made(&core));
}

// 前に見ていたページが消えていたら、今日のジャーナルが開く。消えたページは、ないと伝えない。
#[test]
fn the_app_opens_at_today_when_the_page_it_showed_has_gone() {
    let app = App::new();
    app.download("notes.md", "an apple\n");
    let core = app.open(Arc::default());
    core.sync();
    core.navigated("/pages/notes".into());
    drop(core);
    app.remove("notes.md");
    let core = app.open(Arc::default());
    core.sync();

    assert_eq!(core.restore(true), opened());

    wait_until("the journal of today is made", || today_is_made(&core));
}

// 入れたばかりのアプリは、作らないと言われたら、今日のジャーナルを作らず、何も開かない。
#[test]
fn a_new_app_that_is_not_to_make_today_opens_no_screen() {
    let app = App::new();
    let core = app.open(Arc::default());

    let screen = core.restore(false);

    assert!(screen.unopened);
    assert!(!today_is_made(&core));
}

// 前に見ていたページがあるアプリは、作らないと言われても、そのページを開く。
#[test]
fn an_app_that_showed_a_page_opens_it_also_when_it_is_not_to_make_today() {
    let app = App::new();
    app.download("notes.md", "an apple\n");
    let core = app.open(Arc::default());
    core.sync();
    core.navigated("/pages/notes".into());
    drop(core);
    let core = app.open(Arc::default());

    assert_eq!(core.restore(false), opened());

    wait_until("the page shows", || shown(&core).contains("an apple"));
}

// 何も開かなかったアプリは、最初の取り込みのあとに、今日のジャーナルを開く。画面が開いたあとは、何もしない。
#[test]
fn an_app_that_opened_no_screen_opens_the_first_screen_after_the_taking_in() {
    let app = App::new();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let core = app.open(Arc::default());
    core.restore(false);
    app.download_journal(&today, "the weather was fine\n");
    core.sync();

    assert_eq!(core.open_first_screen(), opened());

    wait_until("the journal of today shows", || {
        shown(&core).contains("the weather was fine")
    });
}

// 戻ると前のページが、進むと戻る前のページが開く。ないと伝えていたファイルは、伝えなくなる。
#[test]
fn back_and_forward_open_the_pages_of_the_history_and_forget_the_missing_file() {
    let app = App::new();
    app.download("notes.md", "an apple\n");
    app.download("other.md", "a pear\n");
    let core = app.open(Arc::default());
    core.sync();
    core.navigated("/pages/notes".into());
    core.navigated("/pages/other".into());
    wait_until("the other page shows", || shown(&core).contains("a pear"));
    core.file_at("/assets/lost.pdf".into());

    assert_eq!(core.back(), opened());

    wait_until("the notes show", || shown(&core).contains("an apple"));
    core.file_at("/assets/lost.pdf".into());
    assert_eq!(core.forward(), opened());
    wait_until("the other page shows", || shown(&core).contains("a pear"));
}

// 戻って開いたページは、最後の画面として残る。アプリを開き直すと、そのページが開く。
#[test]
fn the_page_that_back_opened_opens_when_the_app_opens_again() {
    let app = App::new();
    app.download("notes.md", "an apple\n");
    app.download("other.md", "a pear\n");
    let core = app.open(Arc::default());
    core.sync();
    core.navigated("/pages/notes".into());
    core.navigated("/pages/other".into());
    wait_until("the other page shows", || shown(&core).contains("a pear"));

    core.back();
    wait_until("the notes show", || shown(&core).contains("an apple"));
    drop(core);

    let core = app.open(Arc::default());
    core.restore(true);
    wait_until("the notes show again", || shown(&core).contains("an apple"));
    assert!(!shown(&core).contains("a pear"));
}

// 進んで開いたページも、最後の画面として残る。
#[test]
fn the_page_that_forward_opened_opens_when_the_app_opens_again() {
    let app = App::new();
    app.download("notes.md", "an apple\n");
    app.download("other.md", "a pear\n");
    let core = app.open(Arc::default());
    core.sync();
    core.navigated("/pages/notes".into());
    core.navigated("/pages/other".into());
    wait_until("the other page shows", || shown(&core).contains("a pear"));
    core.back();
    wait_until("the notes show", || shown(&core).contains("an apple"));

    core.forward();
    wait_until("the other page shows again", || {
        shown(&core).contains("a pear")
    });
    drop(core);

    let core = app.open(Arc::default());
    core.restore(true);
    wait_until("the other page shows at the start", || {
        shown(&core).contains("a pear")
    });
    assert!(!shown(&core).contains("an apple"));
}

// 画像のファイルは、名前の大文字と小文字や、濁点の分け方（NFD）を問わずに見つかる。フォルダーや、ないファイルは見つからない。
#[test]
fn the_file_of_an_image_is_found_whatever_the_case_and_the_form_of_its_name() {
    let app = App::new();
    let assets = app.markdown().join("markdown/assets");
    std::fs::create_dir_all(&assets).unwrap();
    std::fs::write(assets.join("Picture.PNG"), "png").unwrap();
    // "ブログ.png" with the sound marks apart (NFD)
    let apart = "\u{30d5}\u{3099}\u{30ed}\u{30af}\u{3099}.png";
    std::fs::write(assets.join(apart), "png").unwrap();
    let core = app.open(Arc::default());

    let picture = core.image("assets/picture.png".into()).unwrap();
    assert!(picture.ends_with("Picture.PNG"));
    let blog = core.image("assets/\u{30d6}\u{30ed}\u{30b0}.png".into());
    assert!(blog.is_some());
    assert_eq!(core.image("assets/other.png".into()), None);
    assert_eq!(core.image("assets".into()), None);
}

// assetsのファイルのアドレスは、そのファイルを答える。名前の大文字と小文字は問わない。なければ、画面はないことを伝え、次のリンクかページを開いたときに伝えなくなる。
#[test]
fn a_link_to_a_file_gives_the_file_or_the_screen_tells_that_it_is_not_here() {
    let app = App::new();
    let assets = app.markdown().join("markdown/assets");
    std::fs::create_dir_all(&assets).unwrap();
    std::fs::write(assets.join("report.pdf"), "pdf").unwrap();
    let core = app.open(Arc::default());

    assert_eq!(core.file_at("/assets/lost.pdf".into()), None);
    assert_eq!(core.screen().missing_file, Some("assets/lost.pdf".into()));
    let file = core.file_at("/assets/Report.pdf".into()).unwrap();
    assert!(file.ends_with("report.pdf"));
    assert_eq!(core.screen().missing_file, None);
    core.file_at("/assets/lost.pdf".into());
    assert_eq!(core.today().missing_file, None);
}

// コアがブラウザで開くように命じたassetsのファイルのアドレスは、そのファイルを答える。名前は復号して探す。なければ、画面はないことを伝える（ADR 083）。
#[test]
fn the_address_of_a_file_that_the_core_orders_open_gives_the_file() {
    let app = App::new();
    let assets = app.markdown().join("markdown/assets");
    std::fs::create_dir_all(&assets).unwrap();
    std::fs::write(assets.join("report a.pdf"), "pdf").unwrap();
    let core = app.open(Arc::default());

    let file = core.file_at("/assets/report%20a.pdf".into()).unwrap();
    assert!(file.ends_with("report a.pdf"));
    assert_eq!(core.screen().missing_file, None);
    assert_eq!(core.file_at("/assets/lost%20b.pdf".into()), None);
    assert_eq!(core.screen().missing_file, Some("assets/lost b.pdf".into()));
}
