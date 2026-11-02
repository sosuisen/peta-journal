//! Maps pages to file paths inside the markdown directory and back.
//! Layout: `{type}s/{dirPath}{name}.md`, e.g. `docs/proj/a.md`, `journals/2026-09-27.md`.
//! Also decides which files in the folder are ours to look at.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::domain::model::Page;

pub const MD_EXT: &str = ".md";
const CONFLICT_MARKER: &str = ".conflict";

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

/// Files we must never import: temp/partial files from editors and Dropbox,
/// Dropbox conflicted copies, our own conflict backups, and anything not .md.
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
    if base.to_lowercase().contains("conflicted copy") {
        return true;
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
        assert!(is_ignored("docs/a (conflicted copy).md"));
        assert!(is_ignored("docs/.sb-123/a.md"));
        assert!(is_ignored("docs/~$a.md"));
        assert!(is_ignored("docs/a.txt"));
        assert!(!is_ignored("docs/a.md"));
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
