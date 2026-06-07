//! SQLite initialization, migration, and helpers.
//!
//! Schema matches PLAN.md design:
//!   sessions — chat session metadata
//!   messages — stream-json messages stored as JSON blobs
//!   settings — key-value settings store

use rusqlite::{Connection, Result as SqliteResult};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Returns the database directory: %APPDATA%/cc-gui on Windows, ~/.local/share/cc-gui on Linux, ~/Library/Application Support/cc-gui on macOS.
pub fn db_dir() -> PathBuf {
    let base = dirs::data_dir().expect("no data dir");
    base.join("cc-gui")
}

/// Returns the full database path.
pub fn db_path() -> PathBuf {
    db_dir().join("sessions.db")
}

/// Open (or create) the database and run migrations.
pub fn open_db() -> SqliteResult<Connection> {
    let dir = db_dir();
    std::fs::create_dir_all(&dir).expect("failed to create data directory");

    let conn = Connection::open(db_path())?;

    // Enable WAL mode for better concurrent read performance
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

    run_migrations(&conn)?;

    Ok(conn)
}

fn run_migrations(conn: &Connection) -> SqliteResult<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL DEFAULT 'New Chat',
            cli_session_id TEXT,
            cwd TEXT NOT NULL DEFAULT '',
            model TEXT DEFAULT '',
            status TEXT NOT NULL DEFAULT 'idle',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
            role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system')),
            content TEXT NOT NULL,
            token_usage TEXT DEFAULT '{}',
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS approved_scenarios (
            tool_name TEXT NOT NULL,
            pattern TEXT NOT NULL DEFAULT '*',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (tool_name, pattern)
        );
        ",
    )?;

    // Create indexes (CREATE INDEX IF NOT EXISTS is available in SQLite 3.27+)
    conn.execute_batch(
        "
        CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);
        CREATE INDEX IF NOT EXISTS idx_messages_created ON messages(created_at);
        CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated_at);
        ",
    )?;

    Ok(())
}

/// Thread-safe database handle shared across Tauri state.
#[derive(Clone)]
pub struct Db {
    pub conn: Arc<Mutex<Connection>>,
}

impl Db {
    pub fn new() -> SqliteResult<Self> {
        Ok(Db {
            conn: Arc::new(Mutex::new(open_db()?)),
        })
    }
}
