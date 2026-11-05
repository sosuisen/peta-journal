//! Maps pages to file paths inside the markdown directory and back.
//! Layout: `{type}s/{dirPath}{name}.md`, e.g. `docs/proj/a.md`, `journals/2026-09-27.md`.
//! Also decides which files in the folder are ours to look at.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::domain::model::Page;

pub const MD_EXT: &str = ".md";
const CONFLICT_MARKER: &str = ".conflict";
/// What Dropbox writes in the name of a conflicted copy, by the language of its app.
const COPY_MARKERS: [&str; 2] = ["conflicted copy", "競合コピー"];

/// Location of a page's markdown file, decoded from a relative path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageLocation {
    pub page_type: String,
    pub dir_path: String,
    pub name: String,
}

/// Relative path (with '/') for a page.
pub fn to_rel_path_of(page: &Page) -> String {
    to_rel_path(&page.page_type, &page.dir_path, &page.name)
}

pub fn to_rel_path(page_type: &str, dir_path: &str, name: &str) -> String {
    format!("{page_type}s/{dir_path}{name}{MD_EXT}")
}

/// The folder under `root` that holds the markdown file of a page.
pub fn export_folder(root: &Path, page: &Page) -> PathBuf {
    let file = root.join(to_rel_path_of(page));
    file.parent().map(Path::to_path_buf).unwrap_or(file)
}

/// Decodes a relative path into (type, dirPath, name). None if the path is not a page file we manage.
pub fn from_rel_path(rel_path: &str) -> Option<PageLocation> {
    if !rel_path.ends_with(MD_EXT) {
        return None;
    }
    let slash = rel_path.find('/')?;
    if slash == 0 {
        return None;
    }
    let page_type = match &rel_path[..slash] {
        "docs" => "doc",
        "journals" => "journal",
        _ => return None,
    };
    let rest = &rel_path[slash + 1..rel_path.len() - MD_EXT.len()];
    if rest.is_empty() {
        return None;
    }
    let (dir_path, name) = match rest.rfind('/') {
        Some(pos) => (&rest[..=pos], &rest[pos + 1..]),
        None => ("", rest),
    };
    if name.is_empty() {
        return None;
    }
    Some(PageLocation {
        page_type: page_type.to_string(),
        dir_path: dir_path.to_string(),
        name: name.to_string(),
    })
}

/// Relative path of `file` under `root`, always using '/'.
pub fn relativize(root: &Path, file: &Path) -> String {
    let rel = file.strip_prefix(root).unwrap_or(file);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Files we must never import: temp/partial files from editors and Dropbox, names that
/// look like a conflicted copy without being one, our own conflict backups, and anything
/// not .md. A real conflicted copy comes through; see `conflicted_copy_of`.
pub fn is_ignored(rel_path: &str) -> bool {
    if !rel_path.ends_with(MD_EXT) {
        return true;
    }
    if has_ignored_segment(rel_path) {
        return true;
    }
    let name = rel_path.rsplit('/').next().unwrap_or(rel_path);
    let base = &name[..name.len() - MD_EXT.len()];
    if base.contains(CONFLICT_MARKER) {
        return true;
    }
    if has_copy_marker(base) {
        return conflicted_copy_of(rel_path).is_none();
    }
    false
}

/// True when any path segment is a hidden/temp entry: .dropbox, .dropbox.cache, .sb-xxxx, ~$lock files.
pub fn has_ignored_segment(rel_path: &str) -> bool {
    rel_path
        .split('/')
        .filter(|s| !s.is_empty())
        .any(|s| s.starts_with('.') || s.starts_with("~$"))
}

/// The page a file is a conflicted copy of, as Dropbox names them.
pub fn conflicted_copy_of(rel_path: &str) -> Option<PageLocation> {
    if has_ignored_segment(rel_path) {
        return None;
    }
    let (page, note) = split_copy_name(rel_path)?;
    if !has_copy_marker(note) {
        return None;
    }
    from_rel_path(&format!("{page}{MD_EXT}"))
}

/// The path before the last note in brackets, and the note.
fn split_copy_name(rel_path: &str) -> Option<(&str, &str)> {
    let base = rel_path.strip_suffix(MD_EXT)?.strip_suffix(')')?;
    base.rsplit_once(" (")
}

/// The conflicted copy holds a version this device wrote, by the name Dropbox gave it.
pub fn is_copy_of_device(rel_path: &str, device: &str) -> bool {
    let Some((_, note)) = split_copy_name(rel_path) else {
        return false;
    };
    note.to_lowercase()
        .strip_prefix(&device.to_lowercase())
        .is_some_and(|rest| rest.starts_with([' ', '\'', '\u{2019}']))
}

fn has_copy_marker(text: &str) -> bool {
    let text = text.to_lowercase();
    COPY_MARKERS.iter().any(|m| text.contains(m))
}

/// Name for a conflict backup written next to the page file.
pub fn conflict_file_name(name: &str, timestamp: &str) -> String {
    format!("{name}{CONFLICT_MARKER}-{timestamp}{MD_EXT}")
}

pub fn sha256(content: &str) -> String {
    hex::encode(Sha256::digest(content.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(page_type: &str, dir_path: &str, name: &str) -> Page {
        let id = hmtid::Hmtid::parse("2026-09-29-00-00-00-AAAAAAA").unwrap();
        Page::new(id, dir_path, name, page_type, 0)
    }

    // エクスポート先のフォルダは、型のフォルダにdirPathを加えたものである。
    #[test]
    fn the_export_folder_is_the_type_folder_plus_the_dir_path() {
        let root = Path::new("/md");
        assert_eq!(
            export_folder(root, &page("doc", "proj/sub/", "a")),
            root.join("docs").join("proj").join("sub")
        );
        assert_eq!(
            export_folder(root, &page("journal", "", "2026-09-27")),
            root.join("journals")
        );
    }

    // ページの場所は相対パスになり、元に戻る。
    #[test]
    fn a_page_location_becomes_a_relative_path_and_back() {
        assert_eq!(to_rel_path("doc", "proj/", "a"), "docs/proj/a.md");
        let loc = from_rel_path("docs/proj/a.md").unwrap();
        assert_eq!(loc.page_type, "doc");
        assert_eq!(loc.dir_path, "proj/");
        assert_eq!(loc.name, "a");
        let loc = from_rel_path("journals/2026-09-27.md").unwrap();
        assert_eq!(loc.page_type, "journal");
        assert_eq!(loc.dir_path, "");
        assert!(from_rel_path("other/a.md").is_none());
        assert!(from_rel_path("docs/a.txt").is_none());
        assert!(from_rel_path("docs/.md").is_none());
    }

    // 競合ファイル、一時ファイル、Markdown以外のファイルは無視される。
    #[test]
    fn conflict_temp_and_non_markdown_files_are_ignored() {
        assert!(is_ignored("docs/a.conflict-20260927-101500.md"));
        assert!(is_ignored("docs/a conflicted copy.md"));
        assert!(is_ignored("docs/.sb-123/a.md"));
        assert!(is_ignored("docs/~$a.md"));
        assert!(is_ignored("docs/a.txt"));
        assert!(!is_ignored("docs/a.md"));
    }

    // Dropboxのconflicted copyの名前から、元のページの場所が分かり、無視されない。
    #[test]
    fn a_conflicted_copy_names_the_page_it_is_a_version_of() {
        let names = [
            "docs/d/a (PC's conflicted copy 2026-11-05).md",
            "docs/d/a (PC の競合コピー 2026-11-05).md",
            "docs/d/a (Conflicted Copy).md",
        ];
        for name in names {
            assert_eq!(conflicted_copy_of(name), from_rel_path("docs/d/a.md"));
            assert!(!is_ignored(name));
        }
    }

    // コピーの名前のPC名がこのPCと同じか、大文字と小文字を区別せずに分かる。
    #[test]
    fn a_copy_names_the_device_that_wrote_it() {
        let japanese = "docs/a (Polaris の競合コピー 2026-11-05).md";
        let english = "docs/a (Polaris's conflicted copy 2026-11-05).md";
        assert!(is_copy_of_device(japanese, "Polaris"));
        assert!(is_copy_of_device(japanese, "POLARIS"));
        assert!(is_copy_of_device(english, "Polaris"));
        assert!(!is_copy_of_device(japanese, "Pol"));
        assert!(!is_copy_of_device(japanese, "Mac"));
        assert!(!is_copy_of_device("docs/a.md", "docs"));
    }

    // conflicted copyの形でない名前は、コピーとして扱わない。
    #[test]
    fn other_names_are_no_conflicted_copy() {
        let names = [
            "docs/d/a.md",
            "docs/d/a (draft).md",
            "docs/d/a conflicted copy.md",
            "docs/d/a (conflicted copy) old.md",
            "docs/d/ (conflicted copy).md",
            "docs/d/a (conflicted copy).txt",
            "docs/.sb-1/a (conflicted copy).md",
            "other/a (conflicted copy).md",
        ];
        for name in names {
            assert_eq!(conflicted_copy_of(name), None, "{name}");
        }
    }

    // 空文字列のSHA-256は既知のダイジェストである。
    #[test]
    fn sha256_of_an_empty_string_is_the_known_digest() {
        assert_eq!(
            sha256(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
