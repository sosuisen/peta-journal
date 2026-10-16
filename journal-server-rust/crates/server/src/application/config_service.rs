//! `config.json` in the configuration directory.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

#[derive(Debug, Clone)]
pub struct ConfigService {
    config_dir: PathBuf,
}

const CONFIG_FILE_NAME: &str = "config.json";

impl ConfigService {
    pub fn new(config_dir: PathBuf) -> ConfigService {
        ConfigService { config_dir }
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
        let mut config = self.load();
        config.insert("markdownDir".into(), Value::String(markdown_dir.to_string()));
        self.save(&config);
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
        let mut config = self.load();
        config.insert("weekNumbering".into(), Value::String(week_numbering.to_string()));
        self.save(&config);
    }

    pub fn logical_outdent(&self) -> bool {
        self.load()
            .get("logicalOutdent")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    pub fn set_logical_outdent(&self, logical_outdent: bool) {
        let mut config = self.load();
        config.insert("logicalOutdent".into(), Value::Bool(logical_outdent));
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
            let text = serde_json::to_string_pretty(&Value::Object(config.clone())).unwrap_or_default();
            std::fs::write(&path, text)
        });
        match result {
            Ok(_) => tracing::info!("Config saved to {}", path.display()),
            Err(e) => tracing::error!("Failed to save config file: {e}"),
        }
    }
}
