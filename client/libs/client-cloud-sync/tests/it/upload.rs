use std::cell::RefCell;

use client_cloud_sync::{SyncError, Uploader};

use crate::fake::{strings, Fixture};

// アプリで変わったページのファイルは、クラウドへ上がる。
#[test]
fn the_file_of_a_page_that_changed_in_the_app_is_uploaded() {
    let sync = Fixture::new();
    sync.download_first(&[("docs/notes.md", "- a"), ("docs/other.md", "- b")]);
    sync.write("docs/notes.md", "- a2");
    sync.upload().unwrap();
    assert_eq!(sync.cloud.uploaded(), strings(&["docs/notes.md"]));
    assert_eq!(sync.cloud.text("docs/notes.md").as_deref(), Some("- a2"));
}

// アプリで作ったページのファイルは、新しいファイルとして上がる。
#[test]
fn the_file_of_a_page_that_was_made_in_the_app_is_uploaded_as_a_new_file() {
    let sync = Fixture::new();
    sync.download_first(&[("docs/notes.md", "- a")]);
    sync.write("journals/2026-11-12.md", "- today");
    sync.upload().unwrap();
    assert_eq!(
        sync.cloud.text("journals/2026-11-12.md").as_deref(),
        Some("- today")
    );
}

// 上げたファイルは、変わらないかぎり、次の回に上がらない。
#[test]
fn an_uploaded_file_is_not_uploaded_again_while_it_stays_the_same() {
    let sync = Fixture::new();
    sync.download_first(&[("docs/notes.md", "- a")]);
    sync.write("docs/notes.md", "- a2");
    sync.write("docs/new.md", "- new");
    sync.upload().unwrap();
    sync.cloud.forget_uploads();
    sync.upload().unwrap();
    sync.write("docs/notes.md", "- a3");
    sync.upload().unwrap();
    assert_eq!(sync.cloud.uploaded(), strings(&["docs/notes.md"]));
    assert_eq!(sync.cloud.text("docs/notes.md").as_deref(), Some("- a3"));
}

// アプリで変わったsettings/properties.jsonは、クラウドへ上がる。この端末で足したプロパティの定義を、ほかの端末に届けるためである（ADR 046）。
#[test]
fn the_file_of_the_property_definitions_that_changed_in_the_app_is_uploaded() {
    let sync = Fixture::new();
    sync.download_first(&[("settings/properties.json", "{}")]);
    sync.write("settings/properties.json", "{\"definitions\": {}}");
    sync.write("settings/other.json", "{}");
    sync.upload().unwrap();
    assert_eq!(
        sync.cloud.uploaded(),
        strings(&["settings/properties.json"])
    );
    assert_eq!(
        sync.cloud.text("settings/properties.json").as_deref(),
        Some("{\"definitions\": {}}")
    );
}

// Markdownでないファイルは、変わっていても上がらない。
#[test]
fn a_file_that_is_not_markdown_is_not_uploaded() {
    let sync = Fixture::new();
    sync.download_first(&[("assets/picture.png", "png")]);
    sync.write("assets/picture.png", "other");
    sync.write("assets/new.png", "new");
    sync.upload().unwrap();
    assert_eq!(sync.cloud.uploaded(), strings(&[]));
}

// クラウドのファイルが、アプリの知らない版になっていたら、上書きせずにエラーが出る。
#[test]
fn a_file_that_changed_in_the_cloud_meanwhile_is_not_written_over() {
    let sync = Fixture::new();
    sync.download_first(&[("docs/notes.md", "- a")]);
    sync.write("docs/notes.md", "- from the phone");
    sync.cloud.file("docs/notes.md", "- from the desk");
    assert_eq!(
        sync.upload(),
        Err(SyncError::ChangedInCloud("docs/notes.md".into()))
    );
    assert_eq!(
        sync.cloud.text("docs/notes.md").as_deref(),
        Some("- from the desk")
    );
}

// アプリで作ったページと同じ名前のファイルがクラウドにできていたら、上書きせずにエラーが出る。
#[test]
fn a_new_file_does_not_write_over_a_file_that_appeared_in_the_cloud() {
    let sync = Fixture::new();
    sync.download_first(&[("docs/notes.md", "- a")]);
    sync.write("docs/new.md", "- from the phone");
    sync.cloud.file("docs/New.md", "- from the desk");
    assert!(matches!(sync.upload(), Err(SyncError::ChangedInCloud(_))));
    assert_eq!(
        sync.cloud.text("docs/new.md").as_deref(),
        Some("- from the desk")
    );
}

// 途中で失敗しても、それまでに上げたファイルは、次の回に上がらない。
#[test]
fn the_files_uploaded_before_a_failure_are_not_uploaded_again() {
    let sync = Fixture::new();
    sync.download_first(&[("docs/a.md", "- a"), ("docs/b.md", "- b")]);
    sync.write("docs/a.md", "- a2");
    sync.write("docs/b.md", "- b2");
    sync.cloud.break_at(Some("docs/b.md"));
    assert!(sync.upload().is_err());
    sync.cloud.break_at(None);
    sync.cloud.forget_uploads();
    sync.upload().unwrap();
    assert_eq!(sync.cloud.uploaded(), strings(&["docs/b.md"]));
}

// アプリを開き直しても、クラウドと同じ中身のファイルは上がらない。
#[test]
fn nothing_is_uploaded_after_the_app_opens_again() {
    let mut sync = Fixture::new();
    sync.download_first(&[("docs/notes.md", "- a")]);
    sync.write("docs/new.md", "- new");
    sync.upload().unwrap();
    sync.cloud.forget_uploads();
    sync.open();
    sync.upload().unwrap();
    assert_eq!(sync.cloud.uploaded(), strings(&[]));
}

// クラウドで消えたファイルと同じ名前のページをアプリで作ると、新しいファイルとして上がる。
#[test]
fn a_page_made_under_the_name_of_a_file_that_has_gone_is_uploaded_as_a_new_file() {
    let sync = Fixture::new();
    sync.download_first(&[("docs/notes.md", "- a"), ("docs/work/plan.md", "- b")]);
    sync.download_since(
        "c1",
        "c2",
        false,
        vec![
            sync.cloud.gone("docs/Notes.md"),
            sync.cloud.gone("docs/work"),
        ],
    );
    sync.write("docs/notes.md", "- again");
    sync.write("docs/work/plan.md", "- again");
    sync.upload().unwrap();
    assert_eq!(
        sync.cloud.uploaded(),
        strings(&["docs/notes.md", "docs/work/plan.md"])
    );
}

// クラウドが全部を答えたときも、そこにないファイルと同じ名前のページは、新しいファイルとして上がる。
#[test]
fn a_page_made_under_the_name_of_a_file_that_a_whole_list_lacks_is_uploaded_as_a_new_file() {
    let sync = Fixture::new();
    sync.download_first(&[("docs/notes.md", "- a"), ("docs/other.md", "- b")]);
    sync.cloud.gone("docs/notes.md");
    sync.download_since(
        "c1",
        "c2",
        true,
        vec![sync.cloud.file("docs/other.md", "- b")],
    );
    sync.write("docs/notes.md", "- again");
    sync.upload().unwrap();
    assert_eq!(sync.cloud.uploaded(), strings(&["docs/notes.md"]));
}

// 進み具合は、最初に0個、そのあと1個上げるごとに、全体の数と一緒に伝わる。
#[test]
fn the_progress_counts_the_files_to_upload_from_zero_to_all() {
    let sync = Fixture::new();
    sync.download_first(&[
        ("docs/a.md", "- a"),
        ("docs/b.md", "- b"),
        ("docs/c.md", "- c"),
    ]);
    sync.write("docs/a.md", "- a2");
    sync.write("docs/b.md", "- b2");
    let told = RefCell::new(Vec::new());
    Uploader::new(sync.cloud.as_ref(), &sync.local)
        .run(&|done, total| told.borrow_mut().push((done, total)))
        .unwrap();
    assert_eq!(told.into_inner(), vec![(0, 2), (1, 2), (2, 2)]);
}

// 日本語の名前のページも、アプリを開き直したあとに、変わったときだけ上がる。
#[test]
fn a_page_with_a_japanese_name_is_uploaded_only_when_it_changed() {
    let mut sync = Fixture::new();
    sync.download_first(&[("docs/旅行の計画.md", "- a")]);
    sync.open();
    sync.upload().unwrap();
    assert_eq!(sync.cloud.uploaded(), strings(&[]));
    sync.write("docs/旅行の計画.md", "- a2");
    sync.upload().unwrap();
    assert_eq!(sync.cloud.uploaded(), strings(&["docs/旅行の計画.md"]));
}
