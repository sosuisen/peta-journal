//! Folder sync: pages written to their markdown files, and files taken back into pages.

use std::path::PathBuf;

use journal_core::infrastructure::export::reconcile_service::ScanSummary;
use serde_json::json;

use crate::common::items::{field_of_each, vs};
use crate::common::pages::by_the_client;
use crate::common::Core;

impl Core {
    /// The markdown file of a doc page at the root
    fn file_named(&self, title: &str) -> PathBuf {
        self.root.join("docs").join(format!("{title}.md"))
    }

    /// Writes the pages that wait for their files, and gives the file of the page
    fn exported(&self, title: &str) -> String {
        self.core.export.process_whole_queue();
        read(&self.file_named(title))
    }

    /// Scans the folder, as the app does when it is asked to
    fn scan(&self) -> ScanSummary {
        self.core.reconcile.scan()
    }

    /// Gives the page the name with `-renamed` after it, at the root
    fn rename_page(&self, page_id: &str, title: &str) -> String {
        let renamed = format!("{title}-renamed");
        self.core
            .journal
            .update_page(page_id, Some(&renamed), Some(""), false, &by_the_client())
            .unwrap();
        renamed
    }
}

fn read(file: &std::path::Path) -> String {
    std::fs::read_to_string(file).unwrap_or_default()
}

// ページはMarkdownファイルに書き出される。
#[test]
fn a_page_is_written_to_its_markdown_file() {
    let notes = Core::new();
    let page = notes.new_doc_page("sync-export");
    notes.fill(&page, &["first line", "  second line"]);

    let markdown = notes.exported("sync-export");

    assert!(
        markdown.starts_with(&format!("---\nid: {page}\n")),
        "{markdown}"
    );
    assert!(markdown.contains("first line"), "{markdown}");
    assert!(markdown.contains("second line"), "{markdown}");
}

// ファイルへの変更はページに取り込まれる。
#[test]
fn a_change_made_to_the_file_is_merged_into_the_page() {
    let notes = Core::new();
    let page = notes.new_doc_page("sync-import");
    notes.fill(&page, &["before the edit"]);
    let file = notes.exported("sync-import");
    assert!(file.contains("before the edit"), "{file}");

    std::fs::write(
        notes.file_named("sync-import"),
        file.replace("before the edit", "edited on disk, a bit longer"),
    )
    .unwrap();
    let result = notes.scan();

    assert!(result.imported >= 1);
    assert_eq!(
        notes.outline_of(&page),
        vs(&["1:edited on disk, a bit longer"])
    );
}

// ファイルで見出しを普通の文に変えると、取り込んだ項目は見出しの種類を失う。
#[test]
fn a_heading_made_plain_in_the_file_loses_its_heading_type() {
    let notes = Core::new();
    let page = notes.new_doc_page("sync-types");
    let item = crate::common::items::id(&notes.fill(&page, &["# a heading"])[0]);
    assert!(crate::common::items::has_type(
        &notes.item(&item),
        "heading"
    ));
    let file = notes.exported("sync-types");

    std::fs::write(
        notes.file_named("sync-types"),
        file.replace("# a heading", "now a plain line"),
    )
    .unwrap();
    notes.scan();

    let taken = notes.item(&item);
    assert_eq!(taken["body"], json!("now a plain line"));
    assert!(
        !crate::common::items::has_type(&taken, "heading"),
        "{taken}"
    );
}

// ファイルから取り込んだ変更は、ファイルからの変更としてクライアントに知らされる。
#[test]
fn the_clients_hear_of_a_change_taken_in_from_the_file() {
    let notes = Core::new();
    let page = notes.new_doc_page("sync-import-event");
    notes.fill(&page, &["before the edit"]);
    let file = notes.exported("sync-import-event");
    let since = notes.told.messages().len();

    std::fs::write(
        notes.file_named("sync-import-event"),
        file.replace("before the edit", "edited on disk, a bit longer"),
    )
    .unwrap();
    notes.scan();

    let messages = notes.told.messages();
    let event = messages[since..]
        .iter()
        .find(|m| m["type"] == "pagesUpdated" && m["pageId"] == json!(page))
        .unwrap_or_else(|| panic!("no change of the page told: {messages:?}"));
    assert_eq!(event["source"], "file");
}

// ページの名前を変えて書き出すと、古い名前のファイルは消える。
#[test]
fn exporting_a_renamed_page_removes_the_file_of_its_old_name() {
    let notes = Core::new();
    let page = notes.new_doc_page("sync-export-rename");
    notes.exported("sync-export-rename");
    let old_file = notes.file_named("sync-export-rename");
    assert!(old_file.exists());

    let renamed = notes.rename_page(&page, "sync-export-rename");
    notes.exported(&renamed);

    assert!(notes.file_named(&renamed).exists());
    assert!(!old_file.exists());
}

// ページを削除すると、そのファイルも消える。
#[test]
fn deleting_a_page_removes_its_file() {
    let notes = Core::new();
    let page = notes.new_doc_page("sync-export-delete");
    notes.exported("sync-export-delete");
    let file = notes.file_named("sync-export-delete");
    assert!(file.exists());

    notes
        .core
        .journal
        .delete_page(&page, &by_the_client())
        .unwrap();

    assert!(!file.exists());
}

// 名前を変えたページを書き出す前に削除しても、古い名前のファイルは消える。
#[test]
fn deleting_a_renamed_page_before_its_export_removes_the_file_of_its_old_name() {
    let notes = Core::new();
    let page = notes.new_doc_page("sync-export-rename-delete");
    notes.exported("sync-export-rename-delete");
    let old_file = notes.file_named("sync-export-rename-delete");
    assert!(old_file.exists());

    notes.rename_page(&page, "sync-export-rename-delete");
    notes
        .core
        .journal
        .delete_page(&page, &by_the_client())
        .unwrap();

    assert!(!old_file.exists());
}

// フォルダに置いた新しいファイルはページになる。
#[test]
fn a_new_file_in_the_folder_becomes_a_page() {
    let notes = Core::new();
    let docs = notes.root.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(
        docs.join("sync-new.md"),
        "hello from disk\n\nsecond block\n",
    )
    .unwrap();

    let result = notes.scan();

    assert!(result.imported >= 1);
    let found = notes.search_pages("sync-new");
    let ids = field_of_each(&found, "results", "pageId");
    assert_eq!(
        ids.len(),
        1,
        "the new page is found by its file name: {found}"
    );
    assert_eq!(
        notes.outline_of(ids[0].as_str().unwrap()),
        vs(&["1:hello from disk", "1:second block"])
    );
}

// 旧い形のインデックスのファイルは、付け直して書き戻される。
#[test]
fn a_file_with_indexes_of_the_old_scheme_is_reindexed_and_written_back() {
    let notes = Core::new();
    let page = notes.new_doc_page("sync-old-idx");
    notes.fill(&page, &["one", "two"]);
    let file = notes.exported("sync-old-idx");

    // As the server of old wrote it, with a line someone added by hand
    let old = file
        .replace("idx=\"a0\"", "idx=\"z0\"")
        .replace("idx=\"a1\"", "idx=\"z1\"")
        + "\nthree\n";
    assert!(old.contains("idx=\"z1\""), "{old}");
    std::fs::write(notes.file_named("sync-old-idx"), old).unwrap();
    let result = notes.scan();

    assert!(result.imported >= 1);
    assert_eq!(notes.outline_of(&page), vs(&["1:one", "1:two", "1:three"]));
    assert_eq!(notes.idxs_of(&page), vs(&["a0", "a1", "a2"]));
    let written_back = notes.exported("sync-old-idx");
    assert!(
        written_back.contains("three") && !written_back.contains("idx=\"z"),
        "{written_back}"
    );
}

// 長くなりすぎたインデックスのファイルは、付け直して書き戻される。
#[test]
fn a_file_with_an_index_grown_too_long_is_reindexed_and_written_back() {
    let notes = Core::new();
    let page = notes.new_doc_page("sync-long-idx");
    notes.fill(&page, &["one", "two"]);
    let file = notes.exported("sync-long-idx");

    let before = notes.idxs_of(&page);
    let long = format!("{}{}", before[1], "V".repeat(20));
    let grown = file.replace(
        &format!("idx=\"{}\"", before[1]),
        &format!("idx=\"{long}\""),
    );
    assert!(grown.contains(&long), "{grown}");
    std::fs::write(notes.file_named("sync-long-idx"), grown).unwrap();
    let result = notes.scan();

    assert!(result.imported >= 1);
    assert_eq!(notes.outline_of(&page), vs(&["1:one", "1:two"]));
    let idxs = notes.idxs_of(&page);
    assert_eq!(idxs[0], before[0]);
    assert!(idxs[1].len() <= 16, "{idxs:?}");
    let written_back = notes.exported("sync-long-idx");
    assert!(
        written_back.contains(&format!("idx=\"{}\"", idxs[1])) && !written_back.contains(&long),
        "{written_back}"
    );
}

// 変わっていないファイルには手を付けない。
#[test]
fn an_unchanged_file_is_left_alone() {
    let notes = Core::new();
    let page = notes.new_doc_page("sync-unchanged");
    notes.fill(&page, &["stable"]);
    notes.exported("sync-unchanged");
    notes.scan();

    let result = notes.scan();

    assert_eq!(result.imported, 0);
    assert_eq!(notes.outline_of(&page), vs(&["1:stable"]));
}

// docs/もjournals/もないフォルダーは、調べない。フォルダーがつながっていないときに、ページのファイルが消えたと見ないためである。
#[test]
fn a_folder_with_neither_docs_nor_journals_is_not_scanned() {
    let notes = Core::new();
    notes.new_doc_page("sync-unmounted");
    notes.exported("sync-unmounted");
    for dir in ["docs", "journals"] {
        let _ = std::fs::remove_dir_all(notes.root.join(dir));
    }

    assert_eq!(notes.scan(), ScanSummary::default());
}
