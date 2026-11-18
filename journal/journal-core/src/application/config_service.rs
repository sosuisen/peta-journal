//! `config.json` in the configuration directory.

use std::path::{Component, Path, PathBuf};

use parking_lot::Mutex;
use serde_json::{Map, Value};

use crate::infrastructure::files::write_atomic;

#[derive(Debug)]
pub struct ConfigService {
    config_dir: PathBuf,
    /// The folder a throw-away journal stays in: its markdown folder is never outside it,
    /// whatever the config file or a client says (ADR 002). None for the journal of a user.
    confined_to: Option<PathBuf>,
    /// Held over each read and each change of the file. Even with the atomic save, Windows
    /// shows no file for a moment while the rename replaces it. A read in that moment would
    /// find no settings at all.
    file: Mutex<()>,
}

const CONFIG_FILE_NAME: &str = "config.json";
const DEFAULT_TAGS_DIR: &str = "tags/";

impl ConfigService {
    pub fn new(config_dir: PathBuf) -> ConfigService {
        ConfigService {
            config_dir,
            confined_to: None,
            file: Mutex::new(()),
        }
    }

    /// The settings of a throw-away journal, which keeps its markdown folder inside `root`.
    /// Tests start their journal so: a test journal that read a markdown folder from a
    /// config file once wrote its pages over real ones (ADR 002).
    pub fn confined(config_dir: PathBuf, root: PathBuf) -> ConfigService {
        ConfigService {
            config_dir,
            confined_to: Some(root),
            file: Mutex::new(()),
        }
    }

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    fn config_path(&self) -> PathBuf {
        self.config_dir.join(CONFIG_FILE_NAME)
    }

    /// The configured markdown export directory, or an empty string if not configured.
    ///
    /// A throw-away journal always has one: the configured directory when it is inside the
    /// folder of the journal, and else `markdown` in that folder.
    pub fn markdown_dir(&self) -> String {
        let configured = self
            .read()
            .get("markdownDir")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        match &self.confined_to {
            Some(root) if !is_inside(Path::new(&configured), root) => {
                root.join("markdown").to_string_lossy().into_owned()
            }
            _ => configured,
        }
    }

    /// A throw-away journal does not take a directory outside its folder.
    pub fn set_markdown_dir(&self, markdown_dir: &str) {
        if let Some(root) = &self.confined_to {
            if !is_inside(Path::new(markdown_dir), root) {
                tracing::warn!(
                    "The markdown directory stays as it is: {markdown_dir} is outside {}, the folder of this throw-away journal",
                    root.display()
                );
                return;
            }
        }
        self.set("markdownDir", Value::String(markdown_dir.to_string()));
    }

    /// The week numbering system: `us` or `iso` (defaults to `us`).
    pub fn week_numbering(&self) -> String {
        self.read()
            .get("weekNumbering")
            .and_then(|v| v.as_str())
            .unwrap_or("us")
            .to_string()
    }

    pub fn set_week_numbering(&self, week_numbering: &str) {
        self.set("weekNumbering", Value::String(week_numbering.to_string()));
    }

    pub fn logical_outdent(&self) -> bool {
        self.read()
            .get("logicalOutdent")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    pub fn set_logical_outdent(&self, logical_outdent: bool) {
        self.set("logicalOutdent", Value::Bool(logical_outdent));
    }

    /// The directory of the tag pages, relative to the pages root and ending with `/`
    /// (e.g. `tags/`), or an empty string when tag pages live at the root. Without a value
    /// it is `tags/`: the config file stays on each machine, and a machine that never set it
    /// would put the tag pages at the root of the shared folder.
    pub fn tags_dir(&self) -> String {
        self.read()
            .get("tagsDir")
            .and_then(|v| v.as_str())
            .map(normalize_tags_dir)
            .unwrap_or_else(|| DEFAULT_TAGS_DIR.to_string())
    }

    pub fn set_tags_dir(&self, tags_dir: &str) {
        self.set("tagsDir", Value::String(normalize_tags_dir(tags_dir)));
    }

    /// The page the Home button opens, as the target of a wiki link (e.g. `today` or
    /// `a/index`), or an empty string for today's journal.
    pub fn home_link(&self) -> String {
        self.read()
            .get("homeLink")
            .and_then(|v| v.as_str())
            .map(normalize_home_link)
            .unwrap_or_default()
    }

    pub fn set_home_link(&self, home_link: &str) {
        self.set("homeLink", Value::String(normalize_home_link(home_link)));
    }

    fn read(&self) -> Map<String, Value> {
        let _file = self.file.lock();
        self.load()
    }

    /// Changes one setting and keeps the others. The lock keeps a second change from reading
    /// the file before this one is saved, and saving its old copy over this change.
    fn set(&self, key: &str, value: Value) {
        let _file = self.file.lock();
        let mut config = self.load();
        config.insert(key.into(), value);
        self.save(&config);
    }

    fn load(&self) -> Map<String, Value> {
        let path = self.config_path();
        if path.exists() {
            match std::fs::read(&path) {
                Ok(bytes) => match serde_json::from_slice::<Value>(&bytes) {
                    Ok(Value::Object(map)) => return map,
                    Ok(_) => tracing::warn!("Config file is not a JSON object: {}", path.display()),
                    Err(e) => tracing::warn!("Failed to load config file: {e}"),
                },
                Err(e) => tracing::warn!("Failed to load config file: {e}"),
            }
        }
        Map::new()
    }

    fn save(&self, config: &Map<String, Value>) {
        let path = self.config_path();
        let result = std::fs::create_dir_all(&self.config_dir).and_then(|_| {
            let text =
                serde_json::to_string_pretty(&Value::Object(config.clone())).unwrap_or_default();
            write_atomic(&path, &text)
        });
        match result {
            Ok(_) => tracing::info!("Config saved to {}", path.display()),
            Err(e) => tracing::error!("Failed to save config file: {e}"),
        }
    }
}

/// Whether a path is inside a folder, or is the folder. A relative path and a path with
/// `..` are not, as where they lead is not to be read from them.
fn is_inside(path: &Path, root: &Path) -> bool {
    let plain = path.is_absolute() && !path.components().any(|c| c == Component::ParentDir);
    plain && resolved(path).starts_with(resolved(root))
}

/// The path with the links in it followed, as far as it exists. The temporary folder of
/// macOS is behind a link, so the same folder is written in two ways.
fn resolved(path: &Path) -> PathBuf {
    let mut missing = Vec::new();
    let mut existing = path;
    loop {
        if let Ok(real) = existing.canonicalize() {
            return missing
                .iter()
                .rev()
                .fold(real, |path, name| path.join(name));
        }
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                missing.push(name);
                existing = parent;
            }
            _ => return path.to_path_buf(),
        }
    }
}

/// A directory path in the `dirPath` form: no leading `/`, a trailing `/` unless empty.
/// Spaces and slashes around the value are user noise, not part of a directory name.
fn normalize_tags_dir(value: &str) -> String {
    let trimmed = value.trim().trim_matches('/');
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{trimmed}/")
    }
}

/// The target of a wiki link: a value typed as the link itself, `[[a/Foo]]`, is taken as its
/// target, as the link brackets are not part of a page name.
fn normalize_home_link(value: &str) -> String {
    let trimmed = value.trim();
    trimmed
        .strip_prefix("[[")
        .and_then(|v| v.strip_suffix("]]"))
        .unwrap_or(trimmed)
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier};

    use std::path::{Path, PathBuf};

    use super::{is_inside, normalize_home_link, normalize_tags_dir, ConfigService};

    // 使い捨てのジャーナルは、設定がなくても、自分のフォルダーの中のMarkdownフォルダーを持つ。
    #[test]
    fn a_throw_away_journal_has_a_markdown_folder_in_its_own_folder_without_a_setting() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let config = ConfigService::confined(root.join("config"), root.clone());

        assert_eq!(Path::new(&config.markdown_dir()), root.join("markdown"));
    }

    // 使い捨てのジャーナルは、設定ファイルが外のフォルダーを指していても、そこを使わない。
    #[test]
    fn a_throw_away_journal_does_not_use_a_markdown_folder_of_the_config_file_outside_its_folder() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let root = dir.path().join("journal");
        let config_dir = root.join("config");
        std::fs::create_dir_all(&config_dir).unwrap();
        for outside in [
            outside.path().to_path_buf(),
            // Beside the folder, with a name that starts as the folder does
            dir.path().join("journal-notes"),
            root.join("..").join("notes"),
            PathBuf::from("notes"),
        ] {
            std::fs::write(
                config_dir.join("config.json"),
                serde_json::json!({ "markdownDir": outside }).to_string(),
            )
            .unwrap();
            let config = ConfigService::confined(config_dir.clone(), root.clone());

            assert_eq!(
                Path::new(&config.markdown_dir()),
                root.join("markdown"),
                "{}",
                outside.display()
            );
        }
    }

    // フォルダーがまだないときも、`..`で外へ出る道は、外と見る。
    #[test]
    fn a_path_that_leaves_the_folder_with_two_dots_is_outside_also_when_nothing_exists_yet() {
        let root = Path::new("/no-such-folder-of-peta-journal/journal");

        assert!(!is_inside(&root.join("../notes"), root));
        assert!(!is_inside(&root.join("notes/../../notes"), root));
        assert!(is_inside(&root.join("notes"), root));
        assert!(is_inside(root, root));
    }

    // 使い捨てのジャーナルは、自分のフォルダーの中のMarkdownフォルダーなら、設定のとおりに使う。
    #[test]
    fn a_throw_away_journal_uses_a_configured_markdown_folder_inside_its_folder() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let config = ConfigService::confined(root.join("config"), root.clone());
        // Not made yet: the journal makes the folder when it writes the first page
        let notes = root.join("notes").join("mine");

        config.set_markdown_dir(&notes.to_string_lossy());

        assert_eq!(Path::new(&config.markdown_dir()), notes);
    }

    // 使い捨てのジャーナルは、外のフォルダーへの変更を受け付けず、前のフォルダーのままである。
    #[test]
    fn a_throw_away_journal_keeps_its_markdown_folder_when_told_one_outside() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let config = ConfigService::confined(root.join("config"), root.clone());
        let notes = root.join("notes");
        config.set_markdown_dir(&notes.to_string_lossy());

        config.set_markdown_dir(&outside.path().to_string_lossy());

        assert_eq!(Path::new(&config.markdown_dir()), notes);
        // Nor is it in the file, where a journal started the usual way would read it
        let usual = ConfigService::new(root.join("config"));
        assert_eq!(Path::new(&usual.markdown_dir()), notes);
    }

    // 使い捨てのジャーナルは、中から外へ出るリンクをたどった先も、外と見る。
    #[cfg(unix)]
    #[test]
    fn a_throw_away_journal_takes_a_link_that_leads_out_of_its_folder_as_outside() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let link = root.join("link");
        std::os::unix::fs::symlink(outside.path(), &link).unwrap();
        let config = ConfigService::confined(root.join("config"), root.clone());

        config.set_markdown_dir(&link.join("notes").to_string_lossy());

        assert_eq!(Path::new(&config.markdown_dir()), root.join("markdown"));
    }

    // ふつうのジャーナルは、設定のMarkdownフォルダーを、どこであってもそのまま使う。設定がなければ空である。
    #[test]
    fn a_usual_journal_uses_the_configured_markdown_folder_wherever_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let config = ConfigService::new(dir.path().to_path_buf());
        assert_eq!(config.markdown_dir(), "");

        config.set_markdown_dir("/somewhere/else");

        assert_eq!(config.markdown_dir(), "/somewhere/else");
    }

    // 設定を書いている途中に読んでも、ほかの設定は消えない。
    #[test]
    fn a_setting_read_while_another_is_written_is_still_there() {
        let dir = tempfile::tempdir().unwrap();
        let config = Arc::new(ConfigService::new(dir.path().to_path_buf()));
        config.set_markdown_dir("/notes");

        let writer = std::thread::spawn({
            let config = Arc::clone(&config);
            move || {
                for round in 0..200 {
                    config.set_home_link(&format!("home-{round}"));
                }
            }
        });
        let mut read = Vec::new();
        while !writer.is_finished() {
            read.push(config.markdown_dir());
        }
        writer.join().unwrap();

        let wrong: Vec<&String> = read.iter().filter(|dir| *dir != "/notes").collect();
        assert!(
            wrong.is_empty(),
            "{} of {} reads: {wrong:?}",
            wrong.len(),
            read.len()
        );
        assert_eq!(config.markdown_dir(), "/notes");
    }

    // 別々の設定を同時に変えても、両方の変更が残る。
    #[test]
    fn two_settings_changed_at_once_both_stay() {
        let dir = tempfile::tempdir().unwrap();
        let config = Arc::new(ConfigService::new(dir.path().to_path_buf()));

        // The app and an MCP client may change settings at the same moment. One round is
        // seldom enough to meet, so the test runs many.
        for round in 0..100 {
            let start = Arc::new(Barrier::new(2));
            let home = std::thread::spawn({
                let (config, start) = (Arc::clone(&config), Arc::clone(&start));
                move || {
                    start.wait();
                    config.set_home_link(&format!("home-{round}"));
                }
            });
            let tags = std::thread::spawn({
                let (config, start) = (Arc::clone(&config), Arc::clone(&start));
                move || {
                    start.wait();
                    config.set_tags_dir(&format!("tags-{round}"));
                }
            });
            home.join().unwrap();
            tags.join().unwrap();

            assert_eq!(config.home_link(), format!("home-{round}"));
            assert_eq!(config.tags_dir(), format!("tags-{round}/"));
        }
    }

    // ホームリンクはリンク先の形で保存される。
    #[test]
    fn home_link_is_kept_as_the_target_of_a_link() {
        assert_eq!(normalize_home_link(""), "");
        assert_eq!(normalize_home_link(" today "), "today");
        assert_eq!(normalize_home_link("[[a/Foo]]"), "a/Foo");
        assert_eq!(normalize_home_link(" [[ a/Foo ]] "), "a/Foo");
    }

    // タグのディレクトリはdirPathの形に正規化される。
    #[test]
    fn tags_dir_is_normalized_to_the_dir_path_form() {
        assert_eq!(normalize_tags_dir(""), "");
        assert_eq!(normalize_tags_dir("  /  "), "");
        assert_eq!(normalize_tags_dir("tags"), "tags/");
        assert_eq!(normalize_tags_dir("/tags/"), "tags/");
        assert_eq!(normalize_tags_dir(" meta/tags "), "meta/tags/");
    }
}
