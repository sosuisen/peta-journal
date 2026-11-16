use std::sync::Mutex;

use client_cloud_sync::{Change, Downloader, SyncError};

use crate::fake::{strings, texts, Fixture};

// 最初の回は、クラウドのフォルダーのMarkdownをすべて取り、同じパスに置く。
#[test]
fn the_first_run_downloads_every_markdown_file_to_its_path() {
    let sync = Fixture::new();
    sync.download_first(&[
        ("docs/notes.md", "- a"),
        ("journals/2026/2026-11-12.md", "- b"),
    ]);
    assert_eq!(
        sync.texts(),
        texts(&[
            ("docs/notes.md", "- a"),
            ("journals/2026/2026-11-12.md", "- b")
        ])
    );
}

// assetsのフォルダーのファイルは、Markdownでなくても取る。ページの画像である。
#[test]
fn a_file_of_the_assets_folder_is_downloaded() {
    let sync = Fixture::new();
    sync.download_first(&[("Assets/Picture.PNG", "png"), ("docs/Notes.MD", "- a")]);
    assert_eq!(
        sync.texts(),
        texts(&[("Assets/Picture.PNG", "png"), ("docs/Notes.MD", "- a")])
    );
}

// settings/properties.jsonは、Markdownでなくても取る。プロパティの定義を、ほかの端末から受け取るためである（ADR 046）。settingsのほかのファイルは取らない。
#[test]
fn the_file_of_the_property_definitions_is_downloaded() {
    let sync = Fixture::new();
    sync.download_first(&[
        ("settings/properties.json", "{}"),
        ("settings/other.json", "{}"),
        ("settings/properties (desk's conflicted copy).json", "{}"),
    ]);
    assert_eq!(sync.texts(), texts(&[("settings/properties.json", "{}")]));
}

// assetsの外にある、Markdownでないファイルは取らない。
#[test]
fn a_file_that_is_neither_markdown_nor_an_asset_is_not_downloaded() {
    let sync = Fixture::new();
    sync.download_first(&[
        ("docs/picture.png", "png"),
        (".obsidian/app.json", "{}"),
        ("assets.png", "png"),
    ]);
    assert_eq!(sync.texts(), texts(&[]));
}

// 次の回は、前の回のカーソルから後の変更だけを尋ね、変わったファイルだけを取る。
#[test]
fn the_next_run_asks_for_the_changes_since_the_last_and_downloads_only_those() {
    let sync = Fixture::new();
    sync.download_first(&[("docs/a.md", "- a"), ("docs/b.md", "- b")]);
    sync.cloud.forget_downloads();
    sync.download_since(
        "c1",
        "c2",
        false,
        vec![sync.cloud.file("docs/b.md", "- b2")],
    );
    assert_eq!(sync.cloud.asked_with(), vec![None, Some("c1".to_string())]);
    assert_eq!(sync.cloud.downloaded(), strings(&["docs/b.md"]));
    assert_eq!(
        sync.texts(),
        texts(&[("docs/a.md", "- a"), ("docs/b.md", "- b2")])
    );
}

// クラウドで消えたファイルは、アプリのフォルダーからも消える。
#[test]
fn a_file_that_has_gone_from_the_cloud_is_deleted_here() {
    let sync = Fixture::new();
    sync.download_first(&[("docs/a.md", "- a"), ("docs/b.md", "- b")]);
    sync.download_since("c1", "c2", false, vec![Change::Gone("docs/a.md".into())]);
    assert_eq!(sync.texts(), texts(&[("docs/b.md", "- b")]));
}

// クラウドで消えたフォルダーは、中のファイルごと消える。同じ回でその後に来たファイルは残る。
#[test]
fn a_folder_that_has_gone_takes_its_files_with_it_but_not_those_that_came_after() {
    let sync = Fixture::new();
    sync.download_first(&[("docs/work/old.md", "- old")]);
    sync.download_since(
        "c1",
        "c2",
        false,
        vec![
            sync.cloud.file("docs/work/lost.md", "- lost"),
            Change::Gone("docs/work".into()),
            sync.cloud.file("docs/work/new.md", "- new"),
        ],
    );
    assert_eq!(sync.texts(), texts(&[("docs/work/new.md", "- new")]));
}

// 同じ回でほかのファイルが消えても、その前に変わったファイルは取る。
#[test]
fn a_file_that_changed_before_another_has_gone_is_downloaded() {
    let sync = Fixture::new();
    sync.download_first(&[("docs/a.md", "- a"), ("docs/b.md", "- b")]);
    sync.download_since(
        "c1",
        "c2",
        false,
        vec![
            sync.cloud.file("docs/a.md", "- a2"),
            Change::Gone("docs/b.md".into()),
        ],
    );
    assert_eq!(sync.texts(), texts(&[("docs/a.md", "- a2")]));
}

// 名前の大文字と小文字だけが変わったファイルは、新しい名前だけで残る。
#[test]
fn a_file_whose_name_changed_only_in_case_stays_under_the_new_name_only() {
    let sync = Fixture::new();
    sync.download_first(&[("docs/notes.md", "- a")]);
    sync.download_since(
        "c1",
        "c2",
        false,
        vec![sync.cloud.file("Docs/Notes.md", "- a2")],
    );
    assert_eq!(sync.texts(), texts(&[("docs/Notes.md", "- a2")]));
}

// 大文字と小文字が違うパスで消えたと知らされても、そのファイルは消える。
#[test]
fn a_file_is_deleted_by_a_path_in_another_case() {
    let sync = Fixture::new();
    sync.download_first(&[("docs/Notes.md", "- a")]);
    sync.download_since(
        "c1",
        "c2",
        false,
        vec![
            Change::Gone("docs/notes.md".into()),
            Change::Gone("docs/never-here.md".into()),
        ],
    );
    assert_eq!(sync.texts(), texts(&[]));
}

// ダウンロードが失敗すると、エラーが出て、カーソルは進まない。次の回は同じ変更を尋ね、足りないファイルだけを取る。
#[test]
fn after_a_failed_download_the_next_run_asks_again_and_downloads_only_what_is_missing() {
    let sync = Fixture::new();
    let entries = vec![
        sync.cloud.file("docs/a.md", "- a"),
        sync.cloud.file("docs/b.md", "- b"),
    ];
    sync.cloud.answer(None, entries, "c1", true);
    sync.cloud.break_at(Some("docs/b.md"));
    assert!(matches!(sync.download(), Err(SyncError::Offline(_))));
    sync.cloud.break_at(None);
    sync.cloud.forget_downloads();
    sync.download().unwrap();
    assert_eq!(sync.cloud.asked_with(), vec![None, None]);
    assert_eq!(sync.cloud.downloaded(), strings(&["docs/b.md"]));
    assert_eq!(
        sync.texts(),
        texts(&[("docs/a.md", "- a"), ("docs/b.md", "- b")])
    );
}

// 途中で失敗したファイルは、アプリのフォルダーに半分だけ残ることがない。
#[test]
fn a_download_that_failed_leaves_no_file() {
    let sync = Fixture::new();
    sync.cloud
        .answer(None, vec![sync.cloud.file("docs/a.md", "- a")], "c1", true);
    sync.cloud.break_at(Some("docs/a.md"));
    assert!(sync.download().is_err());
    assert_eq!(sync.texts(), texts(&[]));
}

// クラウドがカーソルを忘れて全部を答えたときは、そこにないMarkdownがアプリのフォルダーから消える。
#[test]
fn a_whole_list_deletes_the_markdown_files_that_are_not_in_it() {
    let sync = Fixture::new();
    sync.download_first(&[("docs/a.md", "- a"), ("docs/b.md", "- b")]);
    sync.cloud.forget_downloads();
    sync.download_since("c1", "c2", true, vec![sync.cloud.file("docs/B.md", "- b")]);
    assert_eq!(sync.texts(), texts(&[("docs/b.md", "- b")]));
    assert_eq!(sync.cloud.downloaded(), strings(&[]));
}

// クラウドが全部を答えたときは、そこにないassetsのファイルも消える。
#[test]
fn a_whole_list_deletes_the_asset_files_that_are_not_in_it() {
    let sync = Fixture::new();
    sync.download_first(&[("assets/a.png", "a"), ("assets/b.png", "b")]);
    sync.download_since("c1", "c2", true, vec![sync.cloud.file("assets/b.png", "b")]);
    assert_eq!(sync.texts(), texts(&[("assets/b.png", "b")]));
}

// 進み具合は、最初に0個、そのあと1個取るごとに、全体の数と一緒に伝わる。取らないファイルは数えない。
#[test]
fn the_progress_counts_the_files_to_download_from_zero_to_all() {
    let sync = Fixture::new();
    sync.download_first(&[("docs/a.md", "- a")]);
    let entries = vec![
        sync.cloud.file("docs/a.md", "- a"),
        sync.cloud.file("docs/b.md", "- b"),
        sync.cloud.file("docs/c.md", "- c"),
    ];
    sync.cloud.answer(Some("c1"), entries, "c2", false);
    let told = Mutex::new(Vec::new());
    Downloader::new(sync.cloud.as_ref(), &sync.local)
        .run(&|done, total| told.lock().unwrap().push((done, total)))
        .unwrap();
    assert_eq!(told.into_inner().unwrap(), vec![(0, 2), (1, 2), (2, 2)]);
}

// アプリのファイルを書けないときは、通信の失敗ではない失敗になる。画面は「通信できない」と出さない。
#[test]
fn a_file_of_the_app_that_cannot_be_written_fails_apart_from_the_network() {
    let mut sync = Fixture::new();
    sync.local.cursor = sync.dir.path().join("not there/cursor");
    sync.cloud.answer(None, vec![], "c1", true);
    assert!(matches!(sync.download(), Err(SyncError::Other(_))));
}
