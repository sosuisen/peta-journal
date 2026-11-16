//! What the download does with the files that the cloud lost. A file that changed in the app
//! and is not uploaded yet stays. An app without a record has no such file.

use std::fs;

use crate::fake::{strings, Fixture};

// クラウドで消えたファイルでも、アプリで変えてまだ上げていないものは消えない。新しいファイルとして上がる。
#[test]
fn a_file_that_changed_here_stays_when_the_cloud_lost_it_and_is_uploaded_as_a_new_file() {
    let sync = Fixture::new();
    sync.download_first(&[("docs/notes.md", "- a"), ("docs/other.md", "- b")]);
    sync.write("docs/notes.md", "- from the phone");
    sync.download_since(
        "c1",
        "c2",
        false,
        vec![
            sync.cloud.gone("docs/Notes.md"),
            sync.cloud.gone("docs/other.md"),
        ],
    );
    sync.upload().unwrap();
    assert_eq!(sync.here(), strings(&["docs/notes.md"]));
    assert_eq!(
        sync.cloud.text("docs/notes.md").as_deref(),
        Some("- from the phone")
    );
}

// クラウドでフォルダーごと消えたときも、その中の、アプリで変えたファイルと作ったファイルは残る。
#[test]
fn the_files_that_changed_here_stay_when_the_cloud_lost_their_folder() {
    let sync = Fixture::new();
    sync.download_first(&[
        ("docs/work/plan.md", "- a"),
        ("docs/work/old.md", "- b"),
        ("docs/work/deep/old.md", "- c"),
    ]);
    sync.write("docs/work/plan.md", "- from the phone");
    sync.write("docs/work/new.md", "- made on the phone");
    sync.download_since("c1", "c2", false, vec![sync.cloud.gone("docs/work")]);
    assert_eq!(
        sync.here(),
        strings(&["docs/work/new.md", "docs/work/plan.md"])
    );
    assert!(!sync.root.join("docs/work/deep").exists());
}

// クラウドが全部を答えたときも、そこにないファイルのうち、アプリで変えたものと作ったものは残る。
#[test]
fn the_files_that_changed_here_stay_when_a_whole_list_lacks_them() {
    let sync = Fixture::new();
    sync.download_first(&[
        ("docs/notes.md", "- a"),
        ("docs/old.md", "- b"),
        ("docs/other.md", "- c"),
    ]);
    sync.write("docs/notes.md", "- from the phone");
    sync.write("docs/new.md", "- made on the phone");
    sync.cloud.gone("docs/notes.md");
    sync.cloud.gone("docs/old.md");
    sync.download_since(
        "c1",
        "c2",
        true,
        vec![sync.cloud.file("docs/other.md", "- c")],
    );
    sync.upload().unwrap();
    assert_eq!(
        sync.here(),
        strings(&["docs/new.md", "docs/notes.md", "docs/other.md"])
    );
    assert_eq!(
        sync.cloud.uploaded(),
        strings(&["docs/new.md", "docs/notes.md"])
    );
}

// 記録のないアプリは、カーソルがあっても、クラウドの全部を尋ねる。
#[test]
fn an_app_without_a_record_asks_for_everything_though_it_has_a_cursor() {
    let mut sync = Fixture::new();
    sync.download_first(&[("docs/notes.md", "- a")]);
    fs::remove_file(sync.record_file()).unwrap();
    sync.open();
    sync.cloud.forget_downloads();
    sync.download().unwrap();
    sync.upload().unwrap();
    assert_eq!(sync.cloud.asked_with(), vec![None, None]);
    assert_eq!(sync.cloud.downloaded(), strings(&[]));
    assert_eq!(sync.cloud.uploaded(), strings(&[]));
}

// 記録のないアプリのファイルは、すべてクラウドの写しである。クラウドにないものは消える。
#[test]
fn an_app_without_a_record_deletes_every_file_that_the_cloud_lacks() {
    let mut sync = Fixture::new();
    sync.download_first(&[("docs/notes.md", "- a"), ("docs/other.md", "- b")]);
    fs::remove_file(sync.record_file()).unwrap();
    sync.open();
    sync.cloud.gone("docs/notes.md");
    let entries = vec![sync.cloud.file("docs/other.md", "- b")];
    sync.cloud.answer(None, entries, "c2", true);
    sync.download().unwrap();
    sync.upload().unwrap();
    assert_eq!(sync.here(), strings(&["docs/other.md"]));
    assert_eq!(sync.cloud.uploaded(), strings(&[]));
}

// 読めない記録（ADR 054より前のAndroidのPropertiesの形式）は、記録がないものとして扱う。アプリで変えたファイルも、クラウドの版で上書きされる。
#[test]
fn a_record_that_cannot_be_read_counts_as_no_record() {
    let mut sync = Fixture::new();
    sync.download_first(&[("docs/notes.md", "- a")]);
    fs::write(
        sync.record_file(),
        "docs/notes.md=rev\\:1|hash|docs/notes.md\n",
    )
    .unwrap();
    sync.open();
    sync.write("docs/notes.md", "- from the phone");
    sync.download().unwrap();
    sync.upload().unwrap();
    assert_eq!(sync.cloud.asked_with(), vec![None, None]);
    assert_eq!(
        fs::read_to_string(sync.root.join("docs/notes.md")).unwrap(),
        "- a"
    );
    assert_eq!(sync.cloud.uploaded(), strings(&[]));
}
