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

    /// Configuration directory (config.json, functions/). Default: ~/.peta-journal
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
    #[arg(
        long,
        env = "PETAJOURNAL_IMPORT_DELETE_PROMPT_COOLDOWN",
        default_value_t = 300
    )]
    pub import_delete_prompt_cooldown: u64,

    /// A file with the id of a page whose file is still there may be that page moved, with
    /// the old file not taken away yet. Wait this long for that before taking the file as a
    /// copy (seconds)
    #[arg(long, env = "PETAJOURNAL_IMPORT_COPY_WAIT", default_value_t = 300)]
    pub import_copy_wait: u64,

    /// A file that changed on disk and was not taken in yet is not written over: the export
    /// waits for the import. After this long it keeps the file as a conflict backup and
    /// writes the page (seconds)
    #[arg(long, env = "PETAJOURNAL_EXPORT_OVERWRITE_WAIT", default_value_t = 300)]
    pub export_overwrite_wait: u64,

    /// Allowed CORS origins, comma separated
    #[arg(
        long,
        env = "PETAJOURNAL_CORS_ORIGINS",
        default_value = "http://localhost:4200,http://localhost:5180"
    )]
    pub cors_origins: String,

    /// Folder with the files of the web frontend (the dist/ Trunk writes). Default: the
    /// frontend/ folder beside the server executable; without it no frontend is served
    #[arg(long, env = "PETAJOURNAL_FRONTEND_DIR")]
    pub frontend_dir: Option<PathBuf>,
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
    pub import_copy_wait_secs: u64,
    pub export_overwrite_wait_secs: u64,
    pub cors_origins: Vec<String>,
    /// The folder with the files of the web frontend, the `dist/` Trunk writes. Without it
    /// the server serves no frontend.
    pub frontend_dir: Option<PathBuf>,
}

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// The folder of the web frontend: the given one, or else `frontend/` beside the executable,
/// where a distributed server has it.
fn frontend_dir(given: Option<PathBuf>, executable: std::io::Result<PathBuf>) -> Option<PathBuf> {
    given.or_else(|| Some(executable.ok()?.parent()?.join("frontend")))
}

impl Settings {
    pub fn from_cli(cli: Cli) -> Settings {
        let home = home_dir().join(".peta-journal");
        if cli.data_dir.is_some() && cli.config_dir.is_none() {
            // A throw-away data dir with the user's config.json still exports into the user's
            // markdown folder, so a test or a second instance can overwrite real pages there.
            tracing::warn!(
                "--data-dir is set but --config-dir is not: the markdown directory from {} will be used",
                home.join("config.json").display()
            );
        }
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
            import_copy_wait_secs: cli.import_copy_wait,
            export_overwrite_wait_secs: cli.export_overwrite_wait,
            cors_origins: cli
                .cors_origins
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            frontend_dir: frontend_dir(cli.frontend_dir, std::env::current_exe()),
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
            import_copy_wait_secs: 300,
            export_overwrite_wait_secs: 300,
            cors_origins: vec!["http://localhost:4200".to_string()],
            frontend_dir: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `--frontend-dir`で指定したフォルダが、画面のフォルダになる。
    #[test]
    fn the_frontend_folder_is_the_one_given_with_the_flag() {
        let cli = Cli::parse_from(["journal-server", "--frontend-dir", "some/dist"]);

        let settings = Settings::from_cli(cli);

        assert_eq!(settings.frontend_dir, Some(PathBuf::from("some/dist")));
    }

    // 指定がなければ、実行ファイルの隣の`frontend/`が画面のフォルダになる。
    #[test]
    fn without_the_flag_the_frontend_folder_is_the_one_beside_the_executable() {
        let dir = tempfile::tempdir().unwrap();
        let beside = dir.path().join("frontend");
        std::fs::create_dir(&beside).unwrap();

        let found = frontend_dir(None, Ok(dir.path().join("journal-server")));

        assert_eq!(found, Some(beside));
    }
}
