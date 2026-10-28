//! Extension functions: static web apps under `{configDir}/functions/<name>/` with a `config.json`.

use std::path::PathBuf;
use std::sync::Arc;

use super::config_service::ConfigService;

const FUNCTIONS_DIR: &str = "functions";
const CONFIG_FILE: &str = "config.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionFunction {
    pub dir_name: String,
    pub name: String,
}

#[derive(Debug)]
pub struct ExtensionFunctionService {
    config: Arc<ConfigService>,
    functions: Vec<ExtensionFunction>,
}

impl ExtensionFunctionService {
    /// Scans the functions directory once at startup.
    pub fn load(config: Arc<ConfigService>) -> ExtensionFunctionService {
        let dir = config.config_dir().join(FUNCTIONS_DIR);
        let mut found = Vec::new();
        if !dir.is_dir() {
            tracing::info!("Functions directory not found: {}", dir.display());
        } else {
            match std::fs::read_dir(&dir) {
                Ok(entries) => {
                    let mut entries: Vec<_> = entries.flatten().collect();
                    entries.sort_by_key(|e| e.file_name());
                    for entry in entries {
                        let path = entry.path();
                        if !path.is_dir() {
                            continue;
                        }
                        let dir_name = entry.file_name().to_string_lossy().into_owned();
                        let config_path = path.join(CONFIG_FILE);
                        if !config_path.exists() {
                            tracing::warn!("No config.json in function directory: {dir_name}");
                            continue;
                        }
                        match std::fs::read(&config_path)
                            .map_err(|e| e.to_string())
                            .and_then(|b| {
                                serde_json::from_slice::<serde_json::Value>(&b)
                                    .map_err(|e| e.to_string())
                            }) {
                            Ok(json) => {
                                let name = json
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or(&dir_name)
                                    .to_string();
                                found.push(ExtensionFunction { dir_name, name });
                            }
                            Err(e) => {
                                tracing::warn!("Failed to read config.json in {dir_name}: {e}")
                            }
                        }
                    }
                }
                Err(e) => tracing::error!("Failed to scan functions directory: {e}"),
            }
        }
        for f in &found {
            tracing::info!("Loaded extension function: {}", f.name);
        }
        ExtensionFunctionService {
            config,
            functions: found,
        }
    }

    pub fn list_functions(&self) -> &[ExtensionFunction] {
        &self.functions
    }

    pub fn functions_dir(&self) -> PathBuf {
        self.config.config_dir().join(FUNCTIONS_DIR)
    }
}
