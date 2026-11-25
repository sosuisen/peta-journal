//! Server settings. The defaults are those of the Java server's `application.properties`;
//! every value can be overridden by a command line flag or an environment variable
//! (`PETAJOURNAL_DATA_DIR`, `PETAJOURNAL_PORT`, ...).

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use journal_core::application::config_service::ConfigService;

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

    /// Starts a throw-away server, as a test needs it: the data, the search index, the
    /// configuration and the markdown folder are all under this directory (data/, index/,
    /// config/, markdown/), and the markdown folder never leaves it, whatever config.json or
    /// a client says. It takes connections from this machine only. Not to be given together with the four directories above. It has no
    /// environment variable: a variable left set would turn the server of a user into a
    /// throw-away one without a word
    #[arg(long, conflicts_with_all = ["data_dir", "db", "index_dir", "config_dir"])]
    pub throwaway_dir: Option<PathBuf>,

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

    /// Allowed CORS origins, comma separated. The web frontend needs none: the server serves
    /// it from its own origin. The default is the dev server of client/apps/web-app/todo
    #[arg(
        long,
        env = "PETAJOURNAL_CORS_ORIGINS",
        default_value = "http://localhost:5180"
    )]
    pub cors_origins: String,

    /// Folder with the files of the web frontend (the dist/ Trunk writes). Default: the
    /// frontend/ folder beside the server executable; without it no frontend is served
    #[arg(long, env = "PETAJOURNAL_FRONTEND_DIR")]
    pub frontend_dir: Option<PathBuf>,

    /// Without a command the server starts
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Work that is run by hand in place of the server, while the server is stopped.
#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Moves the files of assets/ to names made of an ID, and the links to them (ADR-022).
    /// The options of the server, such as --config-dir, come before this command
    AssetNames {
        step: AssetNamesStep,

        /// The file with the table of old and new names: `plan` writes it, the other steps
        /// read it
        #[arg(long)]
        file: PathBuf,
    },

    /// Writes the markdown file of every page again, so that the files of an older server
    /// get what this server writes, such as the modified date of each item. The database
    /// stays as it is
    WritePages,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum AssetNamesStep {
    /// Writes the table of old and new names to the file. Changes nothing else
    Plan,
    /// Moves the files and the links as the table says
    Apply,
    /// Moves the files and the links back as the table says
    Revert,
}

/// The resolved settings used by the whole server.
#[derive(Debug, Clone)]
pub struct Settings {
    pub port: u16,
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub index_dir: PathBuf,
    pub config_dir: PathBuf,
    /// The directory of a throw-away server, which holds the four above and the markdown
    /// folder. The markdown folder never leaves it (ADR 002). None for the server of a user.
    pub throwaway_dir: Option<PathBuf>,
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
    given.or_else(|| {
        let beside = executable.ok()?.parent()?.join("frontend");
        beside.is_dir().then_some(beside)
    })
}

impl Settings {
    pub fn from_cli(cli: Cli) -> Settings {
        if let Some(root) = cli.throwaway_dir.clone() {
            return Settings {
                throwaway_dir: Some(root.clone()),
                ..Settings::with_folders(
                    cli,
                    root.join("data"),
                    None,
                    root.join("index"),
                    root.join("config"),
                )
            };
        }
        let home = home_dir().join(".peta-journal");
        if cli.data_dir.is_some() && cli.config_dir.is_none() {
            // A throw-away data dir with the user's config.json still exports into the user's
            // markdown folder, so a test or a second instance can overwrite real pages there.
            tracing::warn!(
                "--data-dir is set but --config-dir is not: the markdown directory from {} will be used",
                home.join("config.json").display()
            );
        }
        let data_dir = cli.data_dir.clone().unwrap_or_else(|| home.join("data"));
        let db = cli.db.clone();
        let index_dir = cli.index_dir.clone().unwrap_or_else(|| home.join("index"));
        let config_dir = cli.config_dir.clone().unwrap_or(home);
        Settings::with_folders(cli, data_dir, db, index_dir, config_dir)
    }

    /// The settings of the command line, with the folders already decided.
    fn with_folders(
        cli: Cli,
        data_dir: PathBuf,
        db: Option<PathBuf>,
        index_dir: PathBuf,
        config_dir: PathBuf,
    ) -> Settings {
        let db_path = db.unwrap_or_else(|| data_dir.join("journal.db"));
        Settings {
            port: cli.port,
            data_dir,
            db_path,
            index_dir,
            config_dir,
            throwaway_dir: None,
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

    /// The machines the server takes connections from. A throw-away server is for the tests
    /// and the tools on this machine.
    pub fn reach(&self) -> crate::listen::Reach {
        match self.throwaway_dir {
            Some(_) => crate::listen::Reach::ThisMachine,
            None => crate::listen::Reach::Everywhere,
        }
    }

    /// The settings of the server as the journal reads them from `config.json`.
    pub fn config_service(&self) -> ConfigService {
        match &self.throwaway_dir {
            Some(root) => ConfigService::confined(self.config_dir.clone(), root.clone()),
            None => ConfigService::new(self.config_dir.clone()),
        }
    }

    /// Settings for a throw-away server, which keeps everything under one directory (tests).
    pub fn under(root: &std::path::Path, port: u16) -> Settings {
        Settings {
            port,
            data_dir: root.join("data"),
            db_path: root.join("data").join("journal.db"),
            index_dir: root.join("index"),
            config_dir: root.join("config"),
            throwaway_dir: Some(root.to_path_buf()),
            export_interval_secs: 1,
            import_enabled: true,
            import_debounce_ms: 2000,
            import_scan_interval_secs: 60,
            import_delete_prompt_cooldown_secs: 300,
            import_copy_wait_secs: 300,
            export_overwrite_wait_secs: 300,
            cors_origins: vec!["http://localhost:5180".to_string()],
            frontend_dir: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `--throwaway-dir`だけで、データ、索引、設定のフォルダーが、そのフォルダーの下に決まる。
    #[test]
    fn the_throwaway_folder_alone_places_the_data_the_index_and_the_config_under_it() {
        let cli = Cli::parse_from([
            "journal-server",
            "--throwaway-dir",
            "/tmp/e2e",
            "--port",
            "8187",
        ]);

        let settings = Settings::from_cli(cli);

        let root = PathBuf::from("/tmp/e2e");
        assert_eq!(settings.port, 8187);
        assert_eq!(settings.data_dir, root.join("data"));
        assert_eq!(settings.db_path, root.join("data").join("journal.db"));
        assert_eq!(settings.index_dir, root.join("index"));
        assert_eq!(settings.config_dir, root.join("config"));
        assert_eq!(settings.throwaway_dir, Some(root));
    }

    // `--throwaway-dir`は、データ、DB、索引、設定のフォルダーの指定と一緒には使えない。
    #[test]
    fn the_throwaway_folder_is_refused_together_with_a_folder_of_its_own_kind() {
        for flag in ["--data-dir", "--db", "--index-dir", "--config-dir"] {
            let parsed =
                Cli::try_parse_from(["journal-server", "--throwaway-dir", "/tmp/e2e", flag, "/x"]);

            assert!(parsed.is_err(), "{flag}");
        }
    }

    // 使い捨てのサーバーのMarkdownフォルダーは、設定がなくても、そのフォルダーの下にある。
    #[test]
    fn the_markdown_folder_of_a_throw_away_server_is_under_its_folder_without_a_config() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().into_owned();
        let cli = Cli::parse_from(["journal-server", "--throwaway-dir", &root]);

        let settings = Settings::from_cli(cli);

        assert_eq!(
            PathBuf::from(settings.config_service().markdown_dir()),
            dir.path().join("markdown")
        );
    }

    // 引数のないサーバーは使い捨てではなく、Markdownフォルダーは設定のとおりである。
    #[test]
    fn a_server_without_the_throwaway_folder_reads_the_markdown_folder_as_configured() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.json"),
            r#"{"markdownDir":"/notes"}"#,
        )
        .unwrap();
        let config_dir = dir.path().to_string_lossy().into_owned();
        let cli = Cli::parse_from(["journal-server", "--config-dir", &config_dir]);

        let settings = Settings::from_cli(cli);

        assert_eq!(settings.throwaway_dir, None);
        assert_eq!(settings.config_service().markdown_dir(), "/notes");
    }

    // 使い捨てのサーバーはこの機械からだけ受け、普通のサーバーはどこからでも受ける。
    #[test]
    fn a_throw_away_server_is_reached_from_this_machine_only_and_another_from_everywhere() {
        let throwaway = Settings::from_cli(Cli::parse_from([
            "journal-server",
            "--throwaway-dir",
            "/tmp/e2e",
        ]));
        let ordinary = Settings::from_cli(Cli::parse_from(["journal-server"]));

        assert_eq!(throwaway.reach(), crate::listen::Reach::ThisMachine);
        assert_eq!(ordinary.reach(), crate::listen::Reach::Everywhere);
    }

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

    // 指定がなく、実行ファイルの隣に`frontend/`もなければ、画面のフォルダはない。
    #[test]
    fn without_the_flag_and_a_folder_beside_the_executable_there_is_no_frontend_folder() {
        let dir = tempfile::tempdir().unwrap();

        let found = frontend_dir(None, Ok(dir.path().join("journal-server")));

        assert_eq!(found, None);
    }

    // 指定したフォルダは、実行ファイルの隣の`frontend/`より優先される。
    #[test]
    fn the_given_frontend_folder_comes_before_the_one_beside_the_executable() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("frontend")).unwrap();
        let given = PathBuf::from("some/dist");

        let found = frontend_dir(Some(given.clone()), Ok(dir.path().join("journal-server")));

        assert_eq!(found, Some(given));
    }
}
