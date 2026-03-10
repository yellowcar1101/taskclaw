use rusqlite::params;
use tauri::State;
use uuid::Uuid;

use crate::commands::tasks::DbState;
use crate::types::Context;

#[tauri::command]
pub fn get_contexts(state: State<DbState>) -> Vec<Context> {
    let conn = state.0.lock().unwrap();
    conn.prepare("SELECT id, name, color, position FROM contexts ORDER BY position")
        .and_then(|mut s| {
            s.query_map([], |r| Ok(Context {
                id: r.get(0)?,
                name: r.get(1)?,
                color: r.get(2)?,
                position: r.get(3)?,
            })).map(|rows| rows.filter_map(|r| r.ok()).collect())
        }).unwrap_or_default()
}

#[tauri::command]
pub fn create_context(state: State<DbState>, name: String, color: String) -> Result<Context, String> {
    let conn = state.0.lock().unwrap();
    let id = Uuid::new_v4().to_string();
    let pos: f64 = conn.query_row(
        "SELECT COALESCE(MAX(position), 0) + 1 FROM contexts", [], |r| r.get(0)
    ).unwrap_or(1.0);
    conn.execute(
        "INSERT INTO contexts (id, name, color, position) VALUES (?1,?2,?3,?4)",
        params![id, name, color, pos],
    ).map_err(|e| e.to_string())?;
    Ok(Context { id, name, color, position: pos })
}

#[tauri::command]
pub fn delete_context(state: State<DbState>, id: String) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    conn.execute("DELETE FROM contexts WHERE id=?1", params![id])
        .map(|_| ()).map_err(|e| e.to_string())
}

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
