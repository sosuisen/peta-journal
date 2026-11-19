//! The pages that wait for their markdown files to be written. An app that embeds the core
//! asks whether a page waits and writes them all at once before it syncs (ADR 055).

use crate::common::pages::by_the_client;
use crate::common::Core;

// 変わったページは、ファイルが書かれるまで待っている。書かれたあとは待っていない。ほかのページは待っていない。
#[test]
fn a_page_waits_from_its_change_until_its_file_is_written() {
    let notes = Core::new();
    let pages = &notes.core.page_service;
    let trip = pages.get_or_create_doc(Some("Trip"), None).unwrap().page;
    notes.core.export.process_whole_queue();
    let home = pages.get_or_create_doc(Some("Home"), None).unwrap().page;
    let export = &notes.core.export;

    assert!(export.waits(&home.id));
    assert!(!export.waits(&trip.id));

    export.process_whole_queue();

    assert!(!export.waits(&home.id));
}

// 待ち行列のページがいくつあっても、1回の呼び出しですべてのファイルが書かれる。
#[test]
fn every_page_that_waits_is_written_by_one_call() {
    let notes = Core::new();
    let names: Vec<String> = (0..25).map(|n| format!("Page{n}")).collect();
    for name in &names {
        let pages = &notes.core.page_service;
        pages.get_or_create_doc(Some(name), None).unwrap();
    }

    notes.core.export.process_whole_queue();

    let missing: Vec<&String> = names
        .iter()
        .filter(|name| !notes.root.join(format!("docs/{name}.md")).is_file())
        .collect();
    assert!(missing.is_empty(), "not written: {missing:?}");
}

// 消したページは、ファイルが書かれる前でも待っていない。
#[test]
fn a_deleted_page_waits_no_more() {
    let notes = Core::new();
    let page = notes.new_doc_page("queue-deleted");
    let id = page.parse().unwrap();
    assert!(notes.core.export.waits(&id));

    notes
        .core
        .journal
        .delete_page(&page, &by_the_client())
        .unwrap();

    assert!(!notes.core.export.waits(&id));
}

// Markdownのフォルダに.mdのファイルが1つもなければ、すべてのページが書き出しを待つ。ほかのファイルは数えない。
#[test]
fn a_folder_without_markdown_files_gets_every_page_written() {
    let notes = Core::new();
    let page = notes.new_doc_page("queue-empty-folder").parse().unwrap();
    notes.core.export.process_whole_queue();
    let folder = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(folder.path().join("sub")).unwrap();
    std::fs::write(folder.path().join("sub/notes.txt"), "not markdown").unwrap();

    notes
        .core
        .export
        .export_all_if_empty(&folder.path().to_string_lossy());

    assert!(notes.core.export.waits(&page));
}

// サブフォルダの奥に.mdのファイルが1つでもあれば、ページは書き出しを待たない。
#[test]
fn a_markdown_file_deep_in_a_subfolder_keeps_the_pages_from_being_written() {
    let notes = Core::new();
    let page = notes.new_doc_page("queue-full-folder").parse().unwrap();
    notes.core.export.process_whole_queue();
    let folder = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(folder.path().join("sub/deeper")).unwrap();
    std::fs::write(folder.path().join("sub/deeper/note.md"), "markdown").unwrap();

    notes
        .core
        .export
        .export_all_if_empty(&folder.path().to_string_lossy());

    assert!(!notes.core.export.waits(&page));
}
