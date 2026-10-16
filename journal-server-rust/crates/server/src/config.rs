//! Server settings. The defaults are those of the Java server's `application.properties`;
//! every value can be overridden by a command line flag or an environment variable
//! (`PETAJOURNAL_DATA_DIR`, `PETAJOURNAL_PORT`, ...).

use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[command(name = "journal-server", about = "PetaJournal journal server", version)]
pub struct Cli {
    /// HTTP port
    #[arg(long, env = "PETAJOURNAL_PORT", default_value_t = 8087)]
    pub port: u16,

    /// Data directory (holds journal.db unless --db is given). Default: ~/.peta-journal/data
    #[arg(long, env = "PETAJOURNAL_DATA_DIR")]
    pub data_dir: Option<PathBuf>,

    /// SQLite database file. Default: {data-dir}/journal.db
    #[arg(long, env = "PETAJOURNAL_DATA_DB")]
    pub db: Option<PathBuf>,

    /// Search index directory. Default: ~/.peta-journal/index
    #[arg(long, env = "PETAJOURNAL_INDEX_DIR")]
    pub index_dir: Option<PathBuf>,

    /// Configuration directory (config.json, versions.json, functions/). Default: ~/.peta-journal
    #[arg(long, env = "PETAJOURNAL_CONFIG_DIR")]
    pub config_dir: Option<PathBuf>,

    /// Export scheduler interval in seconds
    #[arg(long, env = "PETAJOURNAL_EXPORT_INTERVAL", default_value_t = 5)]
    pub export_interval: u64,

    /// Folder -> DB import (changes made to the markdown files are merged back into the DB)
    #[arg(long, env = "PETAJOURNAL_IMPORT_ENABLED", default_value_t = true, action = clap::ArgAction::Set)]
    pub import_enabled: bool,

    /// Wait this long after the last file event before reading the file (ms)
    #[arg(long, env = "PETAJOURNAL_IMPORT_DEBOUNCE_MS", default_value_t = 2000)]
    pub import_debounce_ms: u64,

    /// Full folder scan interval in seconds; the first scan runs 10 s after startup
    #[arg(long, env = "PETAJOURNAL_IMPORT_SCAN_INTERVAL", default_value_t = 60)]
    pub import_scan_interval: u64,

    /// While files are missing and undecided, reopen the confirmation page in the browser
    /// on every scan, but not more often than this (seconds)
    #[arg(long, env = "PETAJOURNAL_IMPORT_DELETE_PROMPT_COOLDOWN", default_value_t = 300)]
    pub import_delete_prompt_cooldown: u64,

    /// Allowed CORS origins, comma separated
    #[arg(long, env = "PETAJOURNAL_CORS_ORIGINS", default_value = "http://localhost:4200,http://localhost:5180")]
    pub cors_origins: String,
}

/// The resolved settings used by the whole server.
#[derive(Debug, Clone)]
pub struct Settings {
    pub port: u16,
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub index_dir: PathBuf,
    pub config_dir: PathBuf,
    pub export_interval_secs: u64,
    pub import_enabled: bool,
    pub import_debounce_ms: u64,
    pub import_scan_interval_secs: u64,
    pub import_delete_prompt_cooldown_secs: u64,
    pub cors_origins: Vec<String>,
}

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

impl Settings {
    pub fn from_cli(cli: Cli) -> Settings {
        let home = home_dir().join(".peta-journal");
        let data_dir = cli.data_dir.unwrap_or_else(|| home.join("data"));
        let db_path = cli.db.unwrap_or_else(|| data_dir.join("journal.db"));
        Settings {
            port: cli.port,
            data_dir,
            db_path,
            index_dir: cli.index_dir.unwrap_or_else(|| home.join("index")),
            config_dir: cli.config_dir.unwrap_or(home),
            export_interval_secs: cli.export_interval.max(1),
            import_enabled: cli.import_enabled,
            import_debounce_ms: cli.import_debounce_ms,
            import_scan_interval_secs: cli.import_scan_interval.max(1),
            import_delete_prompt_cooldown_secs: cli.import_delete_prompt_cooldown,
            cors_origins: cli
                .cors_origins
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
        }
    }

    /// Settings for a server that keeps everything under one directory (tests).
    pub fn under(root: &std::path::Path, port: u16) -> Settings {
        Settings {
            port,
            data_dir: root.join("data"),
            db_path: root.join("data").join("journal.db"),
            index_dir: root.join("index"),
            config_dir: root.join("config"),
            export_interval_secs: 1,
            import_enabled: true,
            import_debounce_ms: 2000,
            import_scan_interval_secs: 60,
            import_delete_prompt_cooldown_secs: 300,
            cors_origins: vec!["http://localhost:4200".to_string()],
        }
    }
}
