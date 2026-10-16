//! SQLite connections. Like the Java server, every repository call takes its own connection;
//! idle connections are kept in a small pool. WAL mode and a 30 s busy timeout let several
//! threads work on the database at once; transactions start with `BEGIN IMMEDIATE` so that a
//! writer takes the lock up front instead of failing on its first write.

use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

use anyhow::Context;
use parking_lot::Mutex;
use rusqlite::{Connection, OpenFlags, Transaction, TransactionBehavior};

#[derive(Debug)]
pub struct Db {
    path: PathBuf,
    idle: Mutex<Vec<Connection>>,
}

/// A connection borrowed from the pool; goes back when dropped.
pub struct PooledConn<'a> {
    conn: Option<Connection>,
    db: &'a Db,
}

impl Deref for PooledConn<'_> {
    type Target = Connection;
    fn deref(&self) -> &Connection {
        self.conn.as_ref().expect("connection present")
    }
}

impl DerefMut for PooledConn<'_> {
    fn deref_mut(&mut self) -> &mut Connection {
        self.conn.as_mut().expect("connection present")
    }
}

impl Drop for PooledConn<'_> {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            let mut idle = self.db.idle.lock();
            if idle.len() < 8 {
                idle.push(conn);
            }
        }
    }
}

impl PooledConn<'_> {
    /// Starts a write transaction that takes the database lock right away.
    pub fn immediate(&mut self) -> rusqlite::Result<Transaction<'_>> {
        self.deref_mut()
            .transaction_with_behavior(TransactionBehavior::Immediate)
    }
}

impl Db {
    /// Opens (or creates) the database file, creating its directory first.
    pub fn open(path: &Path) -> anyhow::Result<Db> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Cannot create database directory: {}", parent.display()))?;
        }
        if path.exists() {
            tracing::debug!("Connecting to existing SQLite database: {}", path.display());
        } else {
            tracing::info!("Creating new SQLite database at: {}", path.display());
        }
        let db = Db {
            path: path.to_path_buf(),
            idle: Mutex::new(Vec::new()),
        };
        // Open once so that the file and the WAL mode exist before anything else runs
        let conn = db.connect()?;
        db.idle.lock().push(conn);
        Ok(db)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn connect(&self) -> anyhow::Result<Connection> {
        let conn = Connection::open_with_flags(
            &self.path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .with_context(|| format!("Cannot open SQLite database: {}", self.path.display()))?;
        conn.busy_timeout(std::time::Duration::from_millis(30_000))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "temp_store", "MEMORY")?;
        Ok(conn)
    }

    /// Borrows a connection from the pool, opening a new one when none is idle.
    pub fn conn(&self) -> anyhow::Result<PooledConn<'_>> {
        let idle = self.idle.lock().pop();
        let conn = match idle {
            Some(c) => c,
            None => self.connect()?,
        };
        Ok(PooledConn {
            conn: Some(conn),
            db: self,
        })
    }
}

/// `?,?,?` for an `IN (...)` clause with `n` values.
pub fn placeholders(n: usize) -> String {
    std::iter::repeat_n("?", n).collect::<Vec<_>>().join(",")
}

/// The current time as Unix epoch milliseconds.
pub fn now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
