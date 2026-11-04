//! `config.json` in the configuration directory.

use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde_json::{Map, Value};

#[derive(Debug)]
pub struct ConfigService {
    config_dir: PathBuf,
    update: Mutex<()>,
}

const CONFIG_FILE_NAME: &str = "config.json";
const DEFAULT_TAGS_DIR: &str = "tags/";

impl ConfigService {
    pub fn new(config_dir: PathBuf) -> ConfigService {
        ConfigService {
            config_dir,
            update: Mutex::new(()),
        }
    }

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    fn config_path(&self) -> PathBuf {
        self.config_dir.join(CONFIG_FILE_NAME)
    }

    /// The configured markdown export directory, or an empty string if not configured.
    pub fn markdown_dir(&self) -> String {
        self.load()
            .get("markdownDir")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    }

    pub fn set_markdown_dir(&self, markdown_dir: &str) {
        self.set("markdownDir", Value::String(markdown_dir.to_string()));
    }

    /// The week numbering system: `us` or `iso` (defaults to `us`).
    pub fn week_numbering(&self) -> String {
        self.load()
            .get("weekNumbering")
            .and_then(|v| v.as_str())
            .unwrap_or("us")
            .to_string()
    }

    pub fn set_week_numbering(&self, week_numbering: &str) {
        self.set("weekNumbering", Value::String(week_numbering.to_string()));
    }

    pub fn logical_outdent(&self) -> bool {
        self.load()
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
        self.load()
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
        self.load()
            .get("homeLink")
            .and_then(|v| v.as_str())
            .map(normalize_home_link)
            .unwrap_or_default()
    }

    pub fn set_home_link(&self, home_link: &str) {
        self.set("homeLink", Value::String(normalize_home_link(home_link)));
    }

    /// Changes one setting and keeps the others. The lock keeps a second change from reading
    /// the file before this one is saved, and saving its old copy over this change.
    fn set(&self, key: &str, value: Value) {
        let _update = self.update.lock();
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
            std::fs::write(&path, text)
        });
        match result {
            Ok(_) => tracing::info!("Config saved to {}", path.display()),
            Err(e) => tracing::error!("Failed to save config file: {e}"),
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

    use super::{normalize_home_link, normalize_tags_dir, ConfigService};

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
