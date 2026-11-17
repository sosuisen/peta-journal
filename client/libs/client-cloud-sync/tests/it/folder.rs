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

// 名前の正規化が違っても、同じフォルダーとして見つかる。iPhoneのファイルシステムは、合成済みの文字（NFC）と分けた文字（NFD）を同じ名前とみなす。
#[test]
fn a_folder_is_found_whatever_unicode_form_its_name_has() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().join("markdown");
    // "ブログ" with the sound marks apart (NFD), as an app of the Mac may have written it
    let apart = "\u{30d5}\u{3099}\u{30ed}\u{30af}\u{3099}";
    fs::create_dir_all(root.join("docs").join(apart)).unwrap();
    let folder = client_cloud_sync::MarkdownFolder::new(root.clone(), dir.path().join("temp"));
    // "ブログ" with the sound marks in the letters (NFC), as Dropbox may name it
    folder
        .put("docs/\u{30d6}\u{30ed}\u{30b0}/trip.md", |part| {
            Ok(fs::write(part, "- a")?)
        })
        .unwrap();
    assert_eq!(
        fs::read_to_string(root.join("docs").join(apart).join("trip.md")).unwrap(),
        "- a"
    );
}
