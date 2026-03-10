use rusqlite::{params, Connection};
use std::sync::Mutex;
use tauri::State;
use uuid::Uuid;
use chrono::Utc;

use crate::types::*;

pub struct DbState(pub Mutex<Option<Connection>>);

fn load_flag(conn: &Connection, flag_id: &Option<String>) -> Option<Flag> {
    let id = flag_id.as_ref()?;
    conn.query_row(
        "SELECT id, name, color, position FROM flags WHERE id=?1",
        params![id],
        |r| Ok(Flag { id: r.get(0)?, name: r.get(1)?, color: r.get(2)?, position: r.get(3)? })
    ).ok()
}

fn load_tags(conn: &Connection, task_id: &str) -> Vec<Tag> {
    conn.prepare(
        "SELECT t.id, t.name, t.color FROM tags t JOIN task_tags tt ON t.id=tt.tag_id WHERE tt.task_id=?1"
    ).and_then(|mut s| {
        s.query_map(params![task_id], |r| Ok(Tag { id: r.get(0)?, name: r.get(1)?, color: r.get(2)? }))
         .map(|rows| rows.filter_map(|r| r.ok()).collect())
    }).unwrap_or_default()
}

fn load_email_links(conn: &Connection, task_id: &str) -> Vec<EmailLink> {
    conn.prepare(
        "SELECT id, task_id, link_type, link_data, subject FROM email_links WHERE task_id=?1"
    ).and_then(|mut s| {
        s.query_map(params![task_id], |r| Ok(EmailLink {
            id: r.get(0)?, task_id: r.get(1)?, link_type: r.get(2)?,
            link_data: r.get(3)?, subject: r.get(4)?
        })).map(|rows| rows.filter_map(|r| r.ok()).collect())
    }).unwrap_or_default()
}

#[allow(clippy::too_many_arguments)]
fn build_task(conn: &Connection, id: String, parent_id: Option<String>, caption: String,
    note: String, position: f64, created_at: String, updated_at: String,
    completed_at: Option<String>, start_date: Option<String>, due_date: Option<String>,
    reminder_at: Option<String>, recurrence_rule: Option<String>,
    flag_id: Option<String>, starred: bool, color: Option<String>) -> Task
{
    let has_children: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM tasks WHERE parent_id=?1 AND completed_at IS NULL)",
        params![&id], |r| r.get(0)
    ).unwrap_or(false);
    let flag = load_flag(conn, &flag_id);
    let tags = load_tags(conn, &id);
    let email_links = load_email_links(conn, &id);
    Task { id, parent_id, caption, note, position, created_at, updated_at, completed_at,
           start_date, due_date, reminder_at, recurrence_rule, flag_id, flag,
           starred, color, tags, email_links, has_children }
}

const TASK_SELECT: &str = "SELECT id, parent_id, caption, note, position, created_at, updated_at,
    completed_at, start_date, due_date, reminder_at, recurrence_rule, flag_id, starred, color FROM tasks";

macro_rules! map_task_row {
    ($conn:expr, $row:ident) => {{
        let id: String = $row.get(0)?;
        let parent_id: Option<String> = $row.get(1)?;
        let caption: String = $row.get(2)?;
        let note: String = $row.get(3)?;
        let position: f64 = $row.get(4)?;
        let created_at: String = $row.get(5)?;
        let updated_at: String = $row.get(6)?;
        let completed_at: Option<String> = $row.get(7)?;
        let start_date: Option<String> = $row.get(8)?;
        let due_date: Option<String> = $row.get(9)?;
        let reminder_at: Option<String> = $row.get(10)?;
        let recurrence_rule: Option<String> = $row.get(11)?;
        let flag_id: Option<String> = $row.get(12)?;
        let starred: bool = $row.get(13)?;
        let color: Option<String> = $row.get(14)?;
        Ok(build_task($conn, id, parent_id, caption, note, position, created_at, updated_at,
            completed_at, start_date, due_date, reminder_at, recurrence_rule, flag_id, starred, color))
    }};
}

fn get_task_by_id(conn: &Connection, id: &str) -> Option<Task> {
    conn.query_row(
        &format!("{} WHERE id=?1", TASK_SELECT),
        params![id],
        |r| map_task_row!(conn, r)
    ).ok()
}

#[tauri::command]
pub fn get_tasks(state: State<DbState>, parent_id: Option<String>) -> Vec<Task> {
    let guard = state.0.lock().unwrap();
    let conn = match guard.as_ref() { Some(c) => c, None => return vec![] };
    let sql = format!("{} WHERE parent_id IS ?1 AND completed_at IS NULL ORDER BY position", TASK_SELECT);
    conn.prepare(&sql).and_then(|mut s| {
        s.query_map(params![parent_id], |r| map_task_row!(conn, r))
         .map(|rows| rows.filter_map(|r| r.ok()).collect())
    }).unwrap_or_default()
}

#[tauri::command]
pub fn get_all_tasks_flat(state: State<DbState>, include_completed: Option<bool>) -> Vec<Task> {
    let guard = state.0.lock().unwrap();
    let conn = match guard.as_ref() { Some(c) => c, None => return vec![] };
    let completed_clause = if include_completed.unwrap_or(false) { "" } else { "WHERE completed_at IS NULL" };
    let sql = format!("{} {} ORDER BY position", TASK_SELECT, completed_clause);
    conn.prepare(&sql).and_then(|mut s| {
        s.query_map([], |r| map_task_row!(conn, r))
         .map(|rows| rows.filter_map(|r| r.ok()).collect())
    }).unwrap_or_default()
}

#[tauri::command]
pub fn create_task(state: State<DbState>, input: CreateTaskInput) -> Result<Task, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard.as_ref().ok_or("Database is locked")?;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let position = input.position.unwrap_or_else(|| {
        let max: f64 = conn.query_row(
            "SELECT COALESCE(MAX(position), 0) FROM tasks WHERE parent_id IS ?1",
            params![input.parent_id], |r| r.get(0)
        ).unwrap_or(0.0);
        max + 1000.0
    });
    conn.execute(
        "INSERT INTO tasks (id, parent_id, caption, note, position, created_at, updated_at,
          start_date, due_date, flag_id, starred)
         VALUES (?1,?2,?3,?4,?5,?6,?6,?7,?8,?9,?10)",
        params![id, input.parent_id, input.caption, input.note.unwrap_or_default(),
                position, now, input.start_date, input.due_date, input.flag_id,
                input.starred.unwrap_or(false)],
    ).map_err(|e| e.to_string())?;
    if let Some(tag_ids) = &input.tag_ids {
        for tid in tag_ids {
            conn.execute("INSERT OR IGNORE INTO task_tags (task_id, tag_id) VALUES (?1,?2)",
                params![id, tid]).ok();
        }
    }
    get_task_by_id(conn, &id).ok_or("Task not found after insert".into())
}

#[tauri::command]
pub fn update_task(state: State<DbState>, id: String, input: UpdateTaskInput) -> Result<Task, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard.as_ref().ok_or("Database is locked")?;
    let now = Utc::now().to_rfc3339();
    macro_rules! set_field {
        ($field:expr, $val:expr) => {
            if let Some(v) = $val {
                conn.execute(&format!("UPDATE tasks SET {}=?1, updated_at=?2 WHERE id=?3", $field),
                    params![v, now, id]).ok();
            }
        };
        (nullable $field:expr, $val:expr) => {
            if let Some(v) = $val {
                let val: Option<&str> = if v.is_empty() { None } else { Some(v) };
                conn.execute(&format!("UPDATE tasks SET {}=?1, updated_at=?2 WHERE id=?3", $field),
                    params![val, now, id]).ok();
            }
        };
    }
    set_field!("caption", input.caption.as_deref());
    set_field!("note", input.note.as_deref());
    set_field!(nullable "start_date", input.start_date.as_deref());
    set_field!(nullable "due_date", input.due_date.as_deref());
    set_field!(nullable "reminder_at", input.reminder_at.as_deref());
    set_field!(nullable "recurrence_rule", input.recurrence_rule.as_deref());
    set_field!(nullable "flag_id", input.flag_id.as_deref());
    set_field!("starred", input.starred.as_ref().map(|v| *v as i32));
    set_field!("color", input.color.as_deref());
    if let Some(tag_ids) = &input.tag_ids {
        conn.execute("DELETE FROM task_tags WHERE task_id=?1", params![id]).ok();
        for tid in tag_ids {
            conn.execute("INSERT OR IGNORE INTO task_tags (task_id, tag_id) VALUES (?1,?2)",
                params![id, tid]).ok();
        }
    }
    get_task_by_id(conn, &id).ok_or("Task not found".into())
}

#[tauri::command]
pub fn delete_task(state: State<DbState>, id: String) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard.as_ref().ok_or("Database is locked")?;
    conn.execute("DELETE FROM tasks WHERE id=?1", params![id])
        .map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn complete_task(state: State<DbState>, id: String, completed: bool) -> Result<Task, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard.as_ref().ok_or("Database is locked")?;
    let now = Utc::now().to_rfc3339();
    let completed_at: Option<String> = if completed { Some(now.clone()) } else { None };
    conn.execute("UPDATE tasks SET completed_at=?1, updated_at=?2 WHERE id=?3",
        params![completed_at, now, id]).map_err(|e| e.to_string())?;
    get_task_by_id(conn, &id).ok_or("Task not found".into())
}

#[tauri::command]
pub fn move_task(state: State<DbState>, id: String, new_parent_id: Option<String>, new_position: f64) -> Result<Task, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard.as_ref().ok_or("Database is locked")?;
    let now = Utc::now().to_rfc3339();
    conn.execute("UPDATE tasks SET parent_id=?1, position=?2, updated_at=?3 WHERE id=?4",
        params![new_parent_id, new_position, now, id]).map_err(|e| e.to_string())?;
    get_task_by_id(conn, &id).ok_or("Task not found".into())
}

#[tauri::command]
pub fn reorder_tasks(state: State<DbState>, ids_and_positions: Vec<(String, f64)>) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard.as_ref().ok_or("Database is locked")?;
    let now = Utc::now().to_rfc3339();
    for (id, pos) in ids_and_positions {
        conn.execute("UPDATE tasks SET position=?1, updated_at=?2 WHERE id=?3",
            params![pos, now, id]).ok();
    }
    Ok(())
}

// ── Encryption commands ────────────────────────────────────────────────────────

/// Returns true if the DB file is SQLCipher-encrypted (does not require open connection).
#[tauri::command]
pub fn check_db_encrypted() -> bool {
    crate::db::is_db_encrypted()
}

/// Returns true if the database is currently locked (no open connection).
#[tauri::command]
pub fn is_db_locked(state: State<DbState>) -> bool {
    state.0.lock().unwrap().is_none()
}

/// Unlock an encrypted database with the provided password.
#[tauri::command]
pub fn unlock_db(state: State<DbState>, password: String) -> Result<(), String> {
    let conn = crate::db::open_with_key(&password)
        .map_err(|_| "Wrong password or corrupted database".to_string())?;
    let mut guard = state.0.lock().unwrap();
    *guard = Some(conn);
    Ok(())
}

/// Set or change the database password. Pass empty string to remove encryption.
/// The database must already be unlocked (open connection required).
#[tauri::command]
pub fn set_db_password(state: State<DbState>, new_password: String) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard.as_ref().ok_or("Database is locked")?;
    let escaped = new_password.replace('\'', "''");
    conn.execute_batch(&format!("PRAGMA rekey='{}';", escaped))
        .map_err(|e| e.to_string())
}
