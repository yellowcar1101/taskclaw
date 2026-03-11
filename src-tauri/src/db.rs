use rusqlite::{Connection, Result, params};
use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("cannot resolve exe path");
    let dir = exe.parent().expect("exe has no parent dir");
    let data = dir.join("Data");
    std::fs::create_dir_all(&data).ok();
    data
}

pub fn db_path() -> PathBuf {
    data_dir().join("tasks.db")
}

pub fn open() -> Result<Connection> {
    open_at(&db_path())
}

pub fn open_at(path: &PathBuf) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    migrate(&conn)?;
    Ok(conn)
}

pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS flags (
            id       TEXT PRIMARY KEY,
            name     TEXT NOT NULL,
            color    TEXT NOT NULL DEFAULT '#4A9EFF',
            position REAL NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS tasks (
            id                TEXT PRIMARY KEY,
            parent_id         TEXT REFERENCES tasks(id) ON DELETE CASCADE,
            caption           TEXT NOT NULL,
            note              TEXT NOT NULL DEFAULT '',
            position          REAL NOT NULL DEFAULT 0,
            created_at        TEXT NOT NULL,
            updated_at        TEXT NOT NULL,
            completed_at      TEXT,
            start_date        TEXT,
            due_date          TEXT,
            reminder_at       TEXT,
            recurrence_rule   TEXT,
            flag_id           TEXT REFERENCES flags(id) ON DELETE SET NULL,
            starred           INTEGER NOT NULL DEFAULT 0,
            color             TEXT,
            is_folder         INTEGER NOT NULL DEFAULT 0,
            is_project        INTEGER NOT NULL DEFAULT 0,
            hide_in_views     INTEGER NOT NULL DEFAULT 0,
            subtasks_in_order INTEGER NOT NULL DEFAULT 0,
            inherit_dates     INTEGER NOT NULL DEFAULT 0,
            custom_format     TEXT
        );

        CREATE TABLE IF NOT EXISTS tags (
            id    TEXT PRIMARY KEY,
            name  TEXT NOT NULL UNIQUE,
            color TEXT NOT NULL DEFAULT '#888888'
        );

        CREATE TABLE IF NOT EXISTS task_tags (
            task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            tag_id  TEXT NOT NULL REFERENCES tags(id)  ON DELETE CASCADE,
            PRIMARY KEY (task_id, tag_id)
        );

        CREATE TABLE IF NOT EXISTS email_links (
            id        TEXT PRIMARY KEY,
            task_id   TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            link_type TEXT NOT NULL,
            link_data TEXT NOT NULL,
            subject   TEXT
        );

        CREATE TABLE IF NOT EXISTS saved_views (
            id             TEXT PRIMARY KEY,
            name           TEXT NOT NULL,
            show_completed INTEGER NOT NULL DEFAULT 0,
            group_by       TEXT NOT NULL DEFAULT 'none',
            sort_by        TEXT NOT NULL DEFAULT 'position',
            sort_dir       TEXT NOT NULL DEFAULT 'asc',
            visible_fields TEXT NOT NULL DEFAULT '[]',
            filter_json    TEXT NOT NULL DEFAULT '{}',
            position       REAL NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS app_settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_tasks_parent   ON tasks(parent_id);
        CREATE INDEX IF NOT EXISTS idx_tasks_due      ON tasks(due_date);
        CREATE INDEX IF NOT EXISTS idx_tasks_start    ON tasks(start_date);
        CREATE INDEX IF NOT EXISTS idx_tasks_flag     ON tasks(flag_id);
        CREATE INDEX IF NOT EXISTS idx_tasks_reminder ON tasks(reminder_at);
    ")?;

    // Seed default flags if empty
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM flags", [], |r| r.get(0)
    ).unwrap_or(0);
    if count == 0 {
        let defaults: &[(&str, &str, f64)] = &[
            ("\u{1F534} Urgent",    "#E05C5C", 0.0),
            ("\u{1F7E1} Review",    "#D4A843", 1.0),
            ("\u{1F535} Waiting",   "#4A9EFF", 2.0),
            ("\u{1F7E2} Delegated", "#6ABF69", 3.0),
        ];
        for (name, color, pos) in defaults {
            let id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO flags (id, name, color, position) VALUES (?1, ?2, ?3, ?4)",
                params![id, name, color, pos],
            )?;
        }
    }

    Ok(())
}
