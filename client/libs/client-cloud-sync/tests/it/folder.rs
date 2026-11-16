use std::fs;

use client_cloud_sync::find_without_case;
use tempfile::TempDir;

// ファイルは、大文字と小文字を区別せずに見つかる。見つかるのは、ここでの綴りのファイルである。
#[test]
fn a_file_is_found_however_its_name_is_spelled() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("Docs")).unwrap();
    fs::write(dir.path().join("Docs/Notes.md"), "- a").unwrap();
    assert_eq!(
        find_without_case(dir.path(), "docs/notes.md"),
        Some(dir.path().join("Docs/Notes.md"))
    );
}

// 先頭が/のパスや、/が続くパスでも、ファイルは見つかる。ADR 054より前は、iOSでしか見つからなかった。
#[test]
fn a_path_with_empty_parts_finds_the_file() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("assets")).unwrap();
    fs::write(dir.path().join("assets/a.png"), "png").unwrap();
    assert_eq!(
        find_without_case(dir.path(), "/assets//a.png"),
        Some(dir.path().join("assets/a.png"))
    );
}

// ないファイルは、見つからない。
#[test]
fn a_file_that_is_not_there_is_not_found() {
    let dir = TempDir::new().unwrap();
    assert_eq!(find_without_case(dir.path(), "docs/notes.md"), None);
}
