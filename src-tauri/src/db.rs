use rusqlite::{Connection, Result, params};
use std::path::PathBuf;

pub fn db_path() -> PathBuf {
    // Store next to the exe in a Data/ subfolder — fully portable, no registry/AppData traces.
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let data_dir = exe_dir.join("Data");
    std::fs::create_dir_all(&data_dir).ok();
    data_dir.join("tasks.db")
}

/// Open the database without a key. Fails if the DB is encrypted.
pub fn open() -> Result<Connection> {
    let conn = Connection::open(db_path())?;
    // Verify we can read — this fails for an encrypted DB
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get::<_, i64>(0))?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    migrate(&conn)?;
    Ok(conn)
}

/// Open an encrypted database with the given password.
pub fn open_with_key(key: &str) -> Result<Connection> {
    let conn = Connection::open(db_path())?;
    let escaped = key.replace('\'', "''");
    conn.execute_batch(&format!("PRAGMA key='{}';", escaped))?;
    // Verify key is correct — fails with "file is not a database" if wrong
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get::<_, i64>(0))?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    migrate(&conn)?;
    Ok(conn)
}

/// Returns true if the DB file exists and is SQLCipher-encrypted.
pub fn is_db_encrypted() -> bool {
    let path = db_path();
    if !path.exists() {
        return false;
    }
    let conn = match Connection::open(&path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get::<_, i64>(0))
        .is_err()
}

pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS flags (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            color TEXT NOT NULL DEFAULT '#FF6B6B',
            position REAL NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            parent_id TEXT,
            caption TEXT NOT NULL,
            note TEXT NOT NULL DEFAULT '',
            position REAL NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            completed_at TEXT,
            start_date TEXT,
            due_date TEXT,
            reminder_at TEXT,
            recurrence_rule TEXT,
            flag_id TEXT,
            starred INTEGER NOT NULL DEFAULT 0,
            color TEXT,
            FOREIGN KEY (parent_id) REFERENCES tasks(id) ON DELETE CASCADE,
            FOREIGN KEY (flag_id) REFERENCES flags(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS tags (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            color TEXT NOT NULL DEFAULT '#888888'
        );

        CREATE TABLE IF NOT EXISTS task_tags (
            task_id TEXT NOT NULL,
            tag_id TEXT NOT NULL,
            PRIMARY KEY (task_id, tag_id),
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
            FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS email_links (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            link_type TEXT NOT NULL,
            link_data TEXT NOT NULL,
            subject TEXT,
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS saved_views (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            show_completed INTEGER NOT NULL DEFAULT 0,
            group_by TEXT NOT NULL DEFAULT 'none',
            sort_by TEXT NOT NULL DEFAULT 'position',
            sort_dir TEXT NOT NULL DEFAULT 'asc',
            visible_fields TEXT NOT NULL DEFAULT '[]',
            position REAL NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_tasks_parent ON tasks(parent_id);
        CREATE INDEX IF NOT EXISTS idx_tasks_due ON tasks(due_date);
        CREATE INDEX IF NOT EXISTS idx_tasks_start ON tasks(start_date);
        CREATE INDEX IF NOT EXISTS idx_tasks_flag ON tasks(flag_id);
    ")?;

    // Add columns to existing installs (idempotent — fails silently if already exists)
    for sql in [
        "ALTER TABLE tasks ADD COLUMN start_date TEXT",
        "ALTER TABLE tasks ADD COLUMN flag_id TEXT REFERENCES flags(id) ON DELETE SET NULL",
        "ALTER TABLE tasks ADD COLUMN reminder_at TEXT",
        "ALTER TABLE tasks ADD COLUMN recurrence_rule TEXT",
        "ALTER TABLE saved_views ADD COLUMN show_completed INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE saved_views ADD COLUMN group_by TEXT NOT NULL DEFAULT 'none'",
        "ALTER TABLE saved_views ADD COLUMN sort_by TEXT NOT NULL DEFAULT 'position'",
        "ALTER TABLE saved_views ADD COLUMN sort_dir TEXT NOT NULL DEFAULT 'asc'",
        "ALTER TABLE saved_views ADD COLUMN visible_fields TEXT NOT NULL DEFAULT '[]'",
    ] {
        conn.execute(sql, []).ok();
    }

    // Seed default flags if empty
    let flag_count: i64 = conn.query_row("SELECT COUNT(*) FROM flags", [], |r| r.get(0)).unwrap_or(0);
    if flag_count == 0 {
        let defaults = [
            ("🔴 Urgent",    "#E05C5C", 0.0),
            ("🟡 Review",    "#D4A843", 1.0),
            ("🔵 Waiting",   "#4A9EFF", 2.0),
            ("🟢 Delegated", "#6ABF69", 3.0),
        ];
        for (name, color, pos) in defaults {
            let id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO flags (id, name, color, position) VALUES (?1,?2,?3,?4)",
                params![id, name, color, pos],
            )?;
        }
    }

    Ok(())
}
