use rusqlite::{Connection, Result, params};
use std::path::PathBuf;

pub fn db_path() -> PathBuf {
    let mut p = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("taskclaw");
    std::fs::create_dir_all(&p).ok();
    p.push("tasks.db");
    p
}

pub fn open() -> Result<Connection> {
    let conn = Connection::open(db_path())?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    migrate(&conn)?;
    Ok(conn)
}

pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            parent_id TEXT,
            caption TEXT NOT NULL,
            note TEXT NOT NULL DEFAULT '',
            position REAL NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            completed_at TEXT,
            importance INTEGER NOT NULL DEFAULT 3,
            urgency INTEGER NOT NULL DEFAULT 3,
            effort INTEGER NOT NULL DEFAULT 3,
            due_date TEXT,
            reminder_at TEXT,
            recurrence_rule TEXT,
            starred INTEGER NOT NULL DEFAULT 0,
            color TEXT,
            FOREIGN KEY (parent_id) REFERENCES tasks(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS contexts (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            color TEXT NOT NULL DEFAULT '#4A9EFF',
            position REAL NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS task_contexts (
            task_id TEXT NOT NULL,
            context_id TEXT NOT NULL,
            PRIMARY KEY (task_id, context_id),
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
            FOREIGN KEY (context_id) REFERENCES contexts(id) ON DELETE CASCADE
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
            filter_json TEXT NOT NULL DEFAULT '{}',
            sort_field TEXT NOT NULL DEFAULT 'position',
            sort_dir TEXT NOT NULL DEFAULT 'asc',
            position REAL NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_tasks_parent ON tasks(parent_id);
        CREATE INDEX IF NOT EXISTS idx_tasks_due ON tasks(due_date);
        CREATE INDEX IF NOT EXISTS idx_task_contexts_task ON task_contexts(task_id);
    ")?;

    // Seed default contexts if empty
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM contexts", [], |r| r.get(0)
    ).unwrap_or(0);
    if count == 0 {
        let defaults = [
            ("@home",     "#4A9EFF"),
            ("@work",     "#FF6B6B"),
            ("@computer", "#A8E063"),
            ("@errands",  "#FFA94D"),
            ("@phone",    "#CC5DE8"),
        ];
        for (name, color) in defaults {
            let id = uuid::Uuid::new_v4().to_string();
            let pos = defaults.iter().position(|(n, _)| *n == name).unwrap_or(0) as f64;
            conn.execute(
                "INSERT INTO contexts (id, name, color, position) VALUES (?1, ?2, ?3, ?4)",
                params![id, name, color, pos],
            )?;
        }
    }

    Ok(())
}
