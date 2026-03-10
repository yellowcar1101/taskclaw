use rusqlite::params;
use tauri::State;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use crate::commands::tasks::DbState;
use crate::types::{Flag, Tag, SavedView};

// ── Flags ─────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_flags(state: State<DbState>) -> Vec<Flag> {
    let conn = state.0.lock().unwrap();
    conn.prepare("SELECT id, name, color, position FROM flags ORDER BY position")
        .and_then(|mut s| {
            s.query_map([], |r| Ok(Flag {
                id: r.get(0)?, name: r.get(1)?, color: r.get(2)?, position: r.get(3)?
            })).map(|rows| rows.filter_map(|r| r.ok()).collect())
        }).unwrap_or_default()
}

#[tauri::command]
pub fn create_flag(state: State<DbState>, name: String, color: String) -> Result<Flag, String> {
    let conn = state.0.lock().unwrap();
    let id = Uuid::new_v4().to_string();
    let pos: f64 = conn.query_row(
        "SELECT COALESCE(MAX(position), 0) + 1 FROM flags", [], |r| r.get(0)
    ).unwrap_or(1.0);
    conn.execute("INSERT INTO flags (id, name, color, position) VALUES (?1,?2,?3,?4)",
        params![id, name, color, pos]).map_err(|e| e.to_string())?;
    Ok(Flag { id, name, color, position: pos })
}

#[tauri::command]
pub fn update_flag(state: State<DbState>, id: String, name: String, color: String) -> Result<Flag, String> {
    let conn = state.0.lock().unwrap();
    conn.execute("UPDATE flags SET name=?1, color=?2 WHERE id=?3",
        params![name, color, id]).map_err(|e| e.to_string())?;
    let pos: f64 = conn.query_row("SELECT position FROM flags WHERE id=?1", params![id], |r| r.get(0))
        .unwrap_or(0.0);
    Ok(Flag { id, name, color, position: pos })
}

#[tauri::command]
pub fn delete_flag(state: State<DbState>, id: String) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    // Unlink tasks first
    conn.execute("UPDATE tasks SET flag_id=NULL WHERE flag_id=?1", params![id]).ok();
    conn.execute("DELETE FROM flags WHERE id=?1", params![id])
        .map(|_| ()).map_err(|e| e.to_string())
}

// ── Tags ──────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_tags(state: State<DbState>) -> Vec<Tag> {
    let conn = state.0.lock().unwrap();
    conn.prepare("SELECT id, name, color FROM tags ORDER BY name")
        .and_then(|mut s| {
            s.query_map([], |r| Ok(Tag { id: r.get(0)?, name: r.get(1)?, color: r.get(2)? }))
             .map(|rows| rows.filter_map(|r| r.ok()).collect())
        }).unwrap_or_default()
}

#[tauri::command]
pub fn create_tag(state: State<DbState>, name: String, color: String) -> Result<Tag, String> {
    let conn = state.0.lock().unwrap();
    let id = Uuid::new_v4().to_string();
    conn.execute("INSERT OR IGNORE INTO tags (id, name, color) VALUES (?1,?2,?3)",
        params![id, name, color]).map_err(|e| e.to_string())?;
    Ok(Tag { id, name, color })
}

#[tauri::command]
pub fn delete_tag(state: State<DbState>, id: String) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    conn.execute("DELETE FROM tags WHERE id=?1", params![id])
        .map(|_| ()).map_err(|e| e.to_string())
}

// ── Email links ───────────────────────────────────────────────────────────────

#[tauri::command]
pub fn add_email_link(state: State<DbState>, task_id: String, link_type: String, link_data: String, subject: Option<String>) -> Result<String, String> {
    let conn = state.0.lock().unwrap();
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO email_links (id, task_id, link_type, link_data, subject) VALUES (?1,?2,?3,?4,?5)",
        params![id, task_id, link_type, link_data, subject],
    ).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn delete_email_link(state: State<DbState>, id: String) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    conn.execute("DELETE FROM email_links WHERE id=?1", params![id])
        .map(|_| ()).map_err(|e| e.to_string())
}

// ── Views ─────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ViewPayload {
    pub name: String,
    pub show_completed: bool,
    pub group_by: String,
    pub sort_by: String,
    pub sort_dir: String,
    pub visible_fields: Vec<String>,
}

fn parse_fields(json: &str) -> Vec<String> {
    serde_json::from_str(json).unwrap_or_default()
}

#[tauri::command]
pub fn get_views(state: State<DbState>) -> Vec<SavedView> {
    let conn = state.0.lock().unwrap();
    conn.prepare("SELECT id, name, show_completed, group_by, sort_by, sort_dir, visible_fields, position FROM saved_views ORDER BY position")
        .and_then(|mut s| {
            s.query_map([], |r| {
                let vf_str: String = r.get(6)?;
                Ok(SavedView {
                    id: r.get(0)?, name: r.get(1)?,
                    show_completed: r.get::<_, i64>(2)? != 0,
                    group_by: r.get(3)?, sort_by: r.get(4)?, sort_dir: r.get(5)?,
                    visible_fields: parse_fields(&vf_str),
                    position: r.get(7)?,
                })
            }).map(|rows| rows.filter_map(|r| r.ok()).collect())
        }).unwrap_or_default()
}

#[tauri::command]
pub fn create_view(state: State<DbState>, payload: ViewPayload) -> Result<SavedView, String> {
    let conn = state.0.lock().unwrap();
    let id = Uuid::new_v4().to_string();
    let pos: f64 = conn.query_row(
        "SELECT COALESCE(MAX(position), 0) + 1 FROM saved_views", [], |r| r.get(0)
    ).unwrap_or(1.0);
    let vf = serde_json::to_string(&payload.visible_fields).unwrap_or_default();
    conn.execute(
        "INSERT INTO saved_views (id, name, show_completed, group_by, sort_by, sort_dir, visible_fields, position)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![id, payload.name, payload.show_completed as i64, payload.group_by,
                payload.sort_by, payload.sort_dir, vf, pos],
    ).map_err(|e| e.to_string())?;
    Ok(SavedView {
        id, name: payload.name, show_completed: payload.show_completed,
        group_by: payload.group_by, sort_by: payload.sort_by, sort_dir: payload.sort_dir,
        visible_fields: payload.visible_fields, position: pos,
    })
}

#[tauri::command]
pub fn update_view(state: State<DbState>, id: String, payload: ViewPayload) -> Result<SavedView, String> {
    let conn = state.0.lock().unwrap();
    let vf = serde_json::to_string(&payload.visible_fields).unwrap_or_default();
    conn.execute(
        "UPDATE saved_views SET name=?1, show_completed=?2, group_by=?3, sort_by=?4, sort_dir=?5, visible_fields=?6 WHERE id=?7",
        params![payload.name, payload.show_completed as i64, payload.group_by,
                payload.sort_by, payload.sort_dir, vf, id],
    ).map_err(|e| e.to_string())?;
    let pos: f64 = conn.query_row("SELECT position FROM saved_views WHERE id=?1", params![id], |r| r.get(0))
        .unwrap_or(0.0);
    Ok(SavedView {
        id, name: payload.name, show_completed: payload.show_completed,
        group_by: payload.group_by, sort_by: payload.sort_by, sort_dir: payload.sort_dir,
        visible_fields: payload.visible_fields, position: pos,
    })
}

#[tauri::command]
pub fn delete_view(state: State<DbState>, id: String) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    conn.execute("DELETE FROM saved_views WHERE id=?1", params![id])
        .map(|_| ()).map_err(|e| e.to_string())
}
