use std::fs;

use client_cloud_sync::{CloudFile, CloudRecord};
use tempfile::TempDir;

fn version(path: &str) -> CloudFile {
    CloudFile {
        path: path.to_string(),
        content_hash: "hash".into(),
        remote: "rev:r1".into(),
    }
}

// ADR 054より前のiOSのアプリが書いた記録を、そのまま読む。パスは、大文字と小文字を区別せずに引ける。
#[test]
fn a_record_that_the_ios_shell_wrote_is_read_as_it_is() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("record");
    fs::write(
        &file,
        r#"{"docs/notes.md":{"contentHash":"abc","path":"docs/Notes.md","remote":"rev:r5"}}"#,
    )
    .unwrap();
    let record = CloudRecord::new(&file);
    assert!(record.is_kept());
    assert_eq!(
        record.get("DOCS/notes.md"),
        Some(CloudFile {
            path: "docs/Notes.md".into(),
            content_hash: "abc".into(),
            remote: "rev:r5".into(),
        })
    );
}

// 保存した記録は、iOSのアプリが書いたのと同じ形で、次に開いたときに読める。
#[test]
fn a_saved_record_is_read_again_in_the_form_of_the_ios_shell() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("record");
    let record = CloudRecord::new(&file);
    assert!(!record.is_kept());
    record.put(version("docs/Notes.md"));
    record.save().unwrap();
    assert!(record.is_kept());
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        r#"{"docs/notes.md":{"path":"docs/Notes.md","contentHash":"hash","remote":"rev:r1"}}"#
    );
    assert_eq!(
        CloudRecord::new(&file).get("docs/notes.md"),
        Some(version("docs/Notes.md"))
    );
}

// フォルダーを忘れると、その中のファイルも忘れる。名前がフォルダーの名前で始まるだけのファイルは残る。
#[test]
fn a_folder_that_is_forgotten_takes_its_files_but_not_files_that_only_start_with_its_name() {
    let dir = TempDir::new().unwrap();
    let record = CloudRecord::new(dir.path().join("record"));
    record.put(version("docs/work/plan.md"));
    record.put(version("docs/work.md"));
    record.remove("Docs/Work");
    assert_eq!(record.get("docs/work/plan.md"), None);
    assert_eq!(record.get("docs/work.md"), Some(version("docs/work.md")));
}
