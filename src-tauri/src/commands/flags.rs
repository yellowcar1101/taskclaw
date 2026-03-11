use rusqlite::params;
use tauri::State;
use uuid::Uuid;
use std::collections::HashMap;

use crate::commands::tasks::DbState;
use crate::types::*;

// ── Flags ─────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_flags(state: State<DbState>) -> Vec<Flag> {
    let conn = match state.0.lock() { Ok(c) => c, Err(_) => return vec![] };
    conn.prepare("SELECT id, name, color, position FROM flags ORDER BY position")
        .and_then(|mut s| {
            s.query_map([], |r| Ok(Flag {
                id: r.get(0)?, name: r.get(1)?, color: r.get(2)?, position: r.get(3)?,
            })).map(|rows| rows.filter_map(|r| r.ok()).collect())
        }).unwrap_or_default()
}

#[tauri::command]
pub fn create_flag(state: State<DbState>, name: String, color: String) -> Result<Flag, String> {
    if name.is_empty() { return Err("name cannot be empty".into()); }
    if name.len() > 50 { return Err("name too long (max 50 chars)".into()); }
    validate_color(&color)?;
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let existing: i64 = conn.query_row(
        "SELECT COUNT(*) FROM flags WHERE name=?1", params![name], |r| r.get(0)
    ).unwrap_or(0);
    if existing > 0 { return Err("name already exists".into()); }
    let id = Uuid::new_v4().to_string();
    let pos: f64 = conn.query_row(
        "SELECT COALESCE(MAX(position),0)+1 FROM flags", [], |r| r.get(0)
    ).unwrap_or(1.0);
    conn.execute(
        "INSERT INTO flags (id, name, color, position) VALUES (?1,?2,?3,?4)",
        params![id, name, color, pos],
    ).map_err(|e| e.to_string())?;
    Ok(Flag { id, name, color, position: pos })
}

#[tauri::command]
pub fn update_flag(state: State<DbState>, id: String, name: String, color: String) -> Result<Flag, String> {
    if name.is_empty() { return Err("name cannot be empty".into()); }
    if name.len() > 50 { return Err("name too long (max 50 chars)".into()); }
    validate_color(&color)?;
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let n = conn.execute(
        "UPDATE flags SET name=?1, color=?2 WHERE id=?3",
        params![name, color, id],
    ).map_err(|e| e.to_string())?;
    if n == 0 { return Err("flag not found".into()); }
    conn.query_row(
        "SELECT id, name, color, position FROM flags WHERE id=?1", params![id],
        |r| Ok(Flag { id: r.get(0)?, name: r.get(1)?, color: r.get(2)?, position: r.get(3)? }),
    ).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_flag(state: State<DbState>, id: String) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let n = conn.execute("DELETE FROM flags WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    if n == 0 { Err("flag not found".into()) } else { Ok(()) }
}

#[tauri::command]
pub fn reorder_flags(state: State<DbState>, ids_and_positions: Vec<(String, f64)>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    for (id, pos) in &ids_and_positions {
        let n = conn.execute("UPDATE flags SET position=?1 WHERE id=?2", params![pos, id])
            .map_err(|e| e.to_string())?;
        if n == 0 { return Err(format!("flag {} not found", id)); }
    }
    Ok(())
}

// ── Tags ──────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_tags(state: State<DbState>) -> Vec<Tag> {
    let conn = match state.0.lock() { Ok(c) => c, Err(_) => return vec![] };
    conn.prepare("SELECT id, name, color FROM tags ORDER BY name")
        .and_then(|mut s| {
            s.query_map([], |r| Ok(Tag { id: r.get(0)?, name: r.get(1)?, color: r.get(2)? }))
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
        }).unwrap_or_default()
}

#[tauri::command]
pub fn create_tag(state: State<DbState>, name: String, color: String) -> Result<Tag, String> {
    if name.is_empty() { return Err("name cannot be empty".into()); }
    if name.len() > 50 { return Err("name too long (max 50 chars)".into()); }
    validate_color(&color)?;
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();
    conn.execute("INSERT INTO tags (id, name, color) VALUES (?1,?2,?3)", params![id, name, color])
        .map_err(|e| if e.to_string().contains("UNIQUE") { "name already exists".into() } else { e.to_string() })?;
    Ok(Tag { id, name, color })
}

#[tauri::command]
pub fn update_tag(state: State<DbState>, id: String, name: String, color: String) -> Result<Tag, String> {
    if name.is_empty() { return Err("name cannot be empty".into()); }
    if name.len() > 50 { return Err("name too long (max 50 chars)".into()); }
    validate_color(&color)?;
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let n = conn.execute("UPDATE tags SET name=?1, color=?2 WHERE id=?3", params![name, color, id])
        .map_err(|e| e.to_string())?;
    if n == 0 { Err("tag not found".into()) } else { Ok(Tag { id, name, color }) }
}

#[tauri::command]
pub fn delete_tag(state: State<DbState>, id: String) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let n = conn.execute("DELETE FROM tags WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    if n == 0 { Err("tag not found".into()) } else { Ok(()) }
}

// ── Views ─────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_views(state: State<DbState>) -> Vec<SavedView> {
    let conn = match state.0.lock() { Ok(c) => c, Err(_) => return vec![] };
    conn.prepare(
        "SELECT id, name, show_completed, group_by, sort_by, sort_dir, visible_fields, filter_json, position
         FROM saved_views ORDER BY position"
    ).and_then(|mut s| {
        s.query_map([], |r| {
            let vf_raw: String = r.get(6)?;
            let vf: Vec<String> = serde_json::from_str(&vf_raw).unwrap_or_default();
            Ok(SavedView {
                id: r.get(0)?, name: r.get(1)?,
                show_completed: r.get::<_,i32>(2)? != 0,
                group_by: r.get(3)?, sort_by: r.get(4)?, sort_dir: r.get(5)?,
                visible_fields: vf,
                filter_json: r.get(7)?, position: r.get(8)?,
            })
        }).map(|rows| rows.filter_map(|r| r.ok()).collect())
    }).unwrap_or_default()
}

fn validate_view_payload(p: &ViewPayload) -> Result<(), String> {
    if p.name.is_empty() { return Err("name cannot be empty".into()); }
    if p.name.len() > 100 { return Err("name too long".into()); }
    let valid_group = ["none","flag","due_date","start_date","tag"];
    if !valid_group.contains(&p.group_by.as_str()) { return Err("invalid group_by".into()); }
    let valid_sort = ["position","caption","due_date","start_date","created_at","flag","starred"];
    if !valid_sort.contains(&p.sort_by.as_str()) { return Err("invalid sort_by".into()); }
    if p.sort_dir != "asc" && p.sort_dir != "desc" { return Err("invalid sort_dir".into()); }
    serde_json::from_str::<serde_json::Value>(&p.filter_json)
        .map_err(|_| "filter_json is not valid JSON")?;
    Ok(())
}

#[tauri::command]
pub fn create_view(state: State<DbState>, payload: ViewPayload) -> Result<SavedView, String> {
    validate_view_payload(&payload)?;
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();
    let pos: f64 = conn.query_row(
        "SELECT COALESCE(MAX(position),0)+1 FROM saved_views", [], |r| r.get(0)
    ).unwrap_or(1.0);
    let vf_json = serde_json::to_string(&payload.visible_fields).unwrap_or_else(|_| "[]".into());
    conn.execute(
        "INSERT INTO saved_views (id, name, show_completed, group_by, sort_by, sort_dir, visible_fields, filter_json, position)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![id, payload.name, payload.show_completed as i32, payload.group_by,
                payload.sort_by, payload.sort_dir, vf_json, payload.filter_json, pos],
    ).map_err(|e| e.to_string())?;
    Ok(SavedView {
        id, name: payload.name, show_completed: payload.show_completed,
        group_by: payload.group_by, sort_by: payload.sort_by, sort_dir: payload.sort_dir,
        visible_fields: payload.visible_fields, filter_json: payload.filter_json, position: pos,
    })
}

#[tauri::command]
pub fn update_view(state: State<DbState>, id: String, payload: ViewPayload) -> Result<SavedView, String> {
    validate_view_payload(&payload)?;
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let vf_json = serde_json::to_string(&payload.visible_fields).unwrap_or_else(|_| "[]".into());
    let n = conn.execute(
        "UPDATE saved_views SET name=?1, show_completed=?2, group_by=?3, sort_by=?4,
          sort_dir=?5, visible_fields=?6, filter_json=?7 WHERE id=?8",
        params![payload.name, payload.show_completed as i32, payload.group_by,
                payload.sort_by, payload.sort_dir, vf_json, payload.filter_json, id],
    ).map_err(|e| e.to_string())?;
    if n == 0 { return Err("view not found".into()); }
    let pos: f64 = conn.query_row("SELECT position FROM saved_views WHERE id=?1", params![id], |r| r.get(0))
        .unwrap_or(0.0);
    Ok(SavedView {
        id, name: payload.name, show_completed: payload.show_completed,
        group_by: payload.group_by, sort_by: payload.sort_by, sort_dir: payload.sort_dir,
        visible_fields: payload.visible_fields, filter_json: payload.filter_json, position: pos,
    })
}

#[tauri::command]
pub fn delete_view(state: State<DbState>, id: String) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let n = conn.execute("DELETE FROM saved_views WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    if n == 0 { Err("view not found".into()) } else { Ok(()) }
}

#[tauri::command]
pub fn reorder_views(state: State<DbState>, ids_and_positions: Vec<(String, f64)>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    for (id, pos) in &ids_and_positions {
        let n = conn.execute("UPDATE saved_views SET position=?1 WHERE id=?2", params![pos, id])
            .map_err(|e| e.to_string())?;
        if n == 0 { return Err(format!("view {} not found", id)); }
    }
    Ok(())
}

// ── Email links ───────────────────────────────────────────────────────────────

#[tauri::command]
pub fn add_email_link(
    state: State<DbState>,
    task_id: String, link_type: String, link_data: String, subject: Option<String>,
) -> Result<String, String> {
    let valid_types = ["message_id", "thread_id", "mailto"];
    if !valid_types.contains(&link_type.as_str()) { return Err("invalid link_type".into()); }
    if link_data.is_empty() { return Err("link_data cannot be empty".into()); }
    if link_data.len() > 2000 { return Err("link_data too long".into()); }
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM tasks WHERE id=?1)", params![task_id], |r| r.get(0)
    ).unwrap_or(false);
    if !exists { return Err("task not found".into()); }
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO email_links (id, task_id, link_type, link_data, subject) VALUES (?1,?2,?3,?4,?5)",
        params![id, task_id, link_type, link_data, subject],
    ).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn delete_email_link(state: State<DbState>, id: String) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let n = conn.execute("DELETE FROM email_links WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    if n == 0 { Err("email link not found".into()) } else { Ok(()) }
}

// ── Settings ──────────────────────────────────────────────────────────────────

/// Keys that the frontend is allowed to write via set_setting.
/// Credentials (gdrive_*, api_token) have dedicated commands and are excluded.
const ALLOWED_SETTING_KEYS: &[&str] = &[
    "app_font",
    "app_font_size",
    "app_compact",
    "app_task_color",
    "startup_remember_position",
    "startup_single_instance",
];

/// Keys excluded from get_all_settings — fetched only by their dedicated commands.
const SENSITIVE_SETTING_KEYS: &[&str] = &[
    "gdrive_access_token",
    "gdrive_refresh_token",
    "gdrive_client_secret",
    "api_token",
];

#[tauri::command]
pub fn get_setting(state: State<DbState>, key: String) -> Option<String> {
    let conn = state.0.lock().ok()?;
    conn.query_row("SELECT value FROM app_settings WHERE key=?1", params![key], |r| r.get(0)).ok()
}

#[tauri::command]
pub fn set_setting(state: State<DbState>, key: String, value: String) -> Result<(), String> {
    if !ALLOWED_SETTING_KEYS.contains(&key.as_str()) {
        return Err(format!("unknown or restricted setting key: {}", key));
    }
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1,?2)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key, value],
    ).map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_all_settings(state: State<DbState>) -> HashMap<String, String> {
    let conn = match state.0.lock() {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };
    // Exclude sensitive credentials — those are only accessible via their dedicated commands
    conn.prepare(
        "SELECT key, value FROM app_settings
         WHERE key NOT IN ('gdrive_access_token','gdrive_refresh_token','gdrive_client_secret','api_token')"
    ).and_then(|mut s| {
        s.query_map([], |r| Ok((r.get::<_,String>(0)?, r.get::<_,String>(1)?)))
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
    }).unwrap_or_default()
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn validate_color(c: &str) -> Result<(), String> {
    if c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|ch| ch.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err("color must be #RRGGBB".into())
    }
}
