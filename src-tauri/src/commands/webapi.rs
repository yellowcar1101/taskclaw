/**
 * TaskClaw Web API
 *
 * A lightweight REST API served from within the Tauri app (Rust + tiny HTTP).
 * Binds to 0.0.0.0:PORT (default 7380) — requires an API token to start.
 *
 * Endpoints:
 *   GET  /api/tasks           - all tasks flat
 *   GET  /api/tasks/:id       - single task
 *   GET  /api/flags
 *   GET  /api/tags
 *   GET  /api/views
 *   GET  /api/health          - {"ok":true,"version":"1.0"}
 *
 * Auth: Bearer token required. Set via Preferences → API → Token.
 */

use rusqlite::params;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::State;

use crate::commands::tasks::DbState;
use crate::db;

const MAX_CONNECTIONS: usize = 50;
const READ_TIMEOUT_SECS: u64 = 5;
const MAX_BODY_BYTES: usize = 1_048_576; // 1 MB

// ── Types ──────────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct ApiState {
    db_path: String,
    api_token: Option<String>,
}

// ── Auto-start ────────────────────────────────────────────────────────────────

/// Called at app startup — starts the API server if it was previously enabled.
pub fn autostart_if_enabled() {
    let db_path = db::db_path().to_string_lossy().to_string();
    let conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(_) => return,
    };
    let port_str: Option<String> = conn.query_row(
        "SELECT value FROM app_settings WHERE key='api_port'",
        [], |r| r.get(0),
    ).ok();
    let Some(port_str) = port_str else { return };
    let Ok(port) = port_str.parse::<u16>() else { return };
    let api_token: Option<String> = conn.query_row(
        "SELECT value FROM app_settings WHERE key='api_token'",
        [], |r| r.get(0),
    ).ok();

    // Refuse to auto-start without a token (security requirement)
    if api_token.as_deref().map(|t| t.trim().is_empty()).unwrap_or(true) {
        eprintln!("TaskClaw WebAPI: skipping auto-start — no API token set");
        return;
    }

    drop(conn);
    eprintln!("TaskClaw WebAPI auto-starting on port {}", port);
    spawn_listener(db_path, port, api_token);
}

fn spawn_listener(db_path: String, port: u16, api_token: Option<String>) {
    let api_state = ApiState { db_path, api_token };
    let active_conns = Arc::new(AtomicUsize::new(0));
    thread::spawn(move || {
        let addr = format!("0.0.0.0:{}", port);
        let listener = match TcpListener::bind(&addr) {
            Ok(l) => l,
            Err(e) => { eprintln!("WebAPI: bind error {}: {}", addr, e); return; }
        };
        eprintln!("TaskClaw WebAPI listening on {}", addr);
        for stream in listener.incoming() {
            match stream {
                Ok(s) => {
                    // Enforce connection limit
                    let prev = active_conns.fetch_add(1, Ordering::SeqCst);
                    if prev >= MAX_CONNECTIONS {
                        active_conns.fetch_sub(1, Ordering::SeqCst);
                        // drop stream — connection silently refused
                        continue;
                    }
                    s.set_read_timeout(Some(Duration::from_secs(READ_TIMEOUT_SECS))).ok();
                    let state = api_state.clone();
                    let conns = active_conns.clone();
                    thread::spawn(move || {
                        handle_request(s, &state);
                        conns.fetch_sub(1, Ordering::SeqCst);
                    });
                }
                Err(_) => {}
            }
        }
    });
}

// ── Commands ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn webapi_start(state: State<DbState>, port: u16) -> Result<String, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;

    let api_token: Option<String> = conn.query_row(
        "SELECT value FROM app_settings WHERE key='api_token'",
        [], |r| r.get(0)
    ).ok();

    // Require a non-empty token before starting
    if api_token.as_deref().map(|t| t.trim().is_empty()).unwrap_or(true) {
        return Err("Set an API token before starting the Web API (Preferences → API → Token).".into());
    }

    // Get DB path from connection
    let db_path: String = conn.query_row(
        "PRAGMA database_list", [], |r| r.get(2)
    ).unwrap_or_else(|_| "tasks.db".to_string());

    // Store enabled state
    conn.execute(
        "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('api_port', ?1)",
        params![port.to_string()],
    ).ok();

    drop(conn);
    spawn_listener(db_path, port, api_token);
    Ok(format!("Web API started on port {}", port))
}

#[tauri::command]
pub fn webapi_set_token(state: State<DbState>, token: String) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('api_token', ?1)",
        params![&token],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn webapi_status(state: State<DbState>) -> serde_json::Value {
    let conn = match state.0.lock() {
        Ok(c) => c,
        Err(_) => return serde_json::json!({ "port": null, "has_token": false }),
    };
    let port: Option<String> = conn.query_row(
        "SELECT value FROM app_settings WHERE key='api_port'",
        [], |r| r.get(0)
    ).ok();
    let has_token: bool = conn.query_row(
        "SELECT 1 FROM app_settings WHERE key='api_token' AND value != ''",
        [], |_| Ok(1_i32)
    ).is_ok();
    serde_json::json!({ "port": port, "has_token": has_token })
}

// ── HTTP handler ──────────────────────────────────────────────────────────────

fn handle_request(mut stream: TcpStream, api_state: &ApiState) {
    let reader = BufReader::new(&stream);
    let mut lines = reader.lines();

    // Read first line
    let first = match lines.next().and_then(|l| l.ok()) {
        Some(l) => l,
        None => return,
    };
    let parts: Vec<&str> = first.split_whitespace().collect();
    if parts.len() < 2 { return; }
    let method = parts[0];
    let path = parts[1];

    // Read headers
    let mut content_length = 0usize;
    let mut auth_header = String::new();
    loop {
        let line = match lines.next().and_then(|l| l.ok()) {
            Some(l) => l,
            None => break,
        };
        if line.is_empty() { break; }
        let lc = line.to_lowercase();
        if lc.starts_with("content-length:") {
            content_length = line[15..].trim().parse().unwrap_or(0);
        }
        if lc.starts_with("authorization:") {
            auth_header = line[14..].trim().to_string();
        }
    }

    // Reject oversized requests
    if content_length > MAX_BODY_BYTES {
        respond(&mut stream, 400, "application/json", r#"{"error":"request too large"}"#);
        return;
    }

    // Auth check (always enforced — API cannot start without a token)
    if let Some(ref token) = api_state.api_token {
        let expected = format!("Bearer {}", token);
        if auth_header != expected {
            respond(&mut stream, 401, "application/json", r#"{"error":"unauthorized"}"#);
            return;
        }
    } else {
        // Should never reach here since autostart/webapi_start require a token
        respond(&mut stream, 401, "application/json", r#"{"error":"unauthorized"}"#);
        return;
    }

    // Route
    let response = route(method, path, &api_state.db_path);
    let (status, body) = response;
    respond(&mut stream, status, "application/json", &body);
}

fn respond(stream: &mut TcpStream, status: u16, content_type: &str, body: &str) {
    let status_text = match status {
        200 => "OK", 201 => "Created", 400 => "Bad Request",
        401 => "Unauthorized", 404 => "Not Found", 500 => "Server Error", _ => "OK"
    };
    // CORS: restrict to localhost only (auth-gated API, no cross-origin browser use needed)
    let cors = "Access-Control-Allow-Origin: http://localhost\r\nAccess-Control-Allow-Methods: GET, OPTIONS\r\nAccess-Control-Allow-Headers: Authorization, Content-Type\r\n";
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}; charset=utf-8\r\nContent-Length: {}\r\n{}\r\n{}",
        status, status_text, content_type, body.len(), cors, body
    );
    stream.write_all(response.as_bytes()).ok();
}

fn route(method: &str, path: &str, db_path: &str) -> (u16, String) {
    if path == "/api/health" || path == "/api/health/" {
        return (200, r#"{"ok":true,"version":"1.0","app":"TaskClaw"}"#.to_string());
    }

    let conn = match rusqlite::Connection::open(db_path) {
        Ok(c) => c,
        Err(e) => return (500, format!(r#"{{"error":"DB error: {e}"}}"#)),
    };
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode=WAL;").ok();

    match (method, path) {
        ("GET", "/api/tasks") | ("GET", "/api/tasks/") => {
            let tasks = query_json(&conn,
                "SELECT id, parent_id, caption, note, position, created_at, updated_at,
                 completed_at, start_date, due_date, reminder_at, recurrence_rule,
                 flag_id, starred, is_folder, is_project, hide_in_views
                 FROM tasks WHERE completed_at IS NULL ORDER BY position",
                None
            );
            (200, tasks.to_string())
        }
        ("GET", p) if p.starts_with("/api/tasks/") => {
            let id = &p[11..];
            // Use parameterized query — no string interpolation of user input
            let row = query_json(&conn,
                "SELECT id, parent_id, caption, note, position, created_at, updated_at,
                 completed_at, start_date, due_date, reminder_at, recurrence_rule,
                 flag_id, starred, is_folder, is_project, hide_in_views
                 FROM tasks WHERE id=?1",
                Some(id)
            );
            if row.as_array().map(|a| a.is_empty()).unwrap_or(true) {
                (404, r#"{"error":"not found"}"#.to_string())
            } else {
                (200, row[0].to_string())
            }
        }
        ("GET", "/api/flags") | ("GET", "/api/flags/") => {
            (200, query_json(&conn, "SELECT id, name, color, position FROM flags ORDER BY position", None).to_string())
        }
        ("GET", "/api/tags") | ("GET", "/api/tags/") => {
            (200, query_json(&conn, "SELECT id, name, color FROM tags ORDER BY name", None).to_string())
        }
        ("GET", "/api/views") | ("GET", "/api/views/") => {
            (200, query_json(&conn, "SELECT id, name, show_completed, group_by, sort_by, sort_dir, filter_json FROM saved_views ORDER BY position", None).to_string())
        }
        ("OPTIONS", _) => {
            (200, "".to_string())
        }
        _ => (404, r#"{"error":"not found"}"#.to_string()),
    }
}

/// Execute a SQL query and return results as a JSON array.
/// `param` is an optional single string parameter bound as ?1.
fn query_json(conn: &rusqlite::Connection, sql: &str, param: Option<&str>) -> serde_json::Value {
    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => return serde_json::json!([]),
    };
    let cols: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

    let map_row = |row: &rusqlite::Row| {
        let mut obj = serde_json::Map::new();
        for (i, col) in cols.iter().enumerate() {
            let val: serde_json::Value = match row.get_ref(i).unwrap_or(rusqlite::types::ValueRef::Null) {
                rusqlite::types::ValueRef::Null       => serde_json::Value::Null,
                rusqlite::types::ValueRef::Integer(n) => serde_json::Value::Number(n.into()),
                rusqlite::types::ValueRef::Real(f)    => serde_json::json!(f),
                rusqlite::types::ValueRef::Text(s)    => serde_json::Value::String(String::from_utf8_lossy(s).into()),
                rusqlite::types::ValueRef::Blob(_)    => serde_json::Value::Null,
            };
            obj.insert(col.clone(), val);
        }
        Ok(serde_json::Value::Object(obj))
    };

    let rows: Vec<serde_json::Value> = if let Some(p) = param {
        match stmt.query_map(params![p], map_row) {
            Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
            Err(_) => vec![],
        }
    } else {
        match stmt.query_map([], map_row) {
            Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
            Err(_) => vec![],
        }
    };

    serde_json::Value::Array(rows)
}
