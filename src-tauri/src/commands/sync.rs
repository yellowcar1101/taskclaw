/**
 * TaskClaw GDrive Sync
 *
 * OAuth2 flow (installed app / loopback):
 *   1. gdrive_start_auth()  → returns auth URL; opens browser
 *   2. gdrive_wait_auth()   → starts localhost server, waits for redirect, exchanges code → stores tokens
 *   3. gdrive_sync_push()   → serialises all data → uploads to Drive
 *   4. gdrive_sync_pull()   → downloads from Drive → replaces local DB data
 *
 * Token refresh happens automatically in every push/pull call.
 */

use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::thread;
use std::sync::mpsc;
use std::time::Duration;
use tauri::State;

use crate::commands::tasks::DbState;

const CLIENT_ID: &str = env!("GDRIVE_CLIENT_ID");
const CLIENT_SECRET: &str = env!("GDRIVE_CLIENT_SECRET");
const TOKEN_URI: &str = "https://oauth2.googleapis.com/token";
const DRIVE_SCOPES: &str = "https://www.googleapis.com/auth/drive.file";
const SYNC_FILE_NAME: &str = "taskclaw-sync.json";

// ── Auth URL ──────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct AuthInfo {
    pub url: String,
    pub port: u16,
}

#[tauri::command]
pub fn gdrive_auth_url() -> Result<AuthInfo, String> {
    // Pick an available port
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().unwrap().port();
    drop(listener); // free it so wait_auth can re-bind

    let redirect = format!("http://localhost:{}", port);
    let url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?\
         client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent",
        urlencoding::encode(CLIENT_ID),
        urlencoding::encode(&redirect),
        urlencoding::encode(DRIVE_SCOPES)
    );
    Ok(AuthInfo { url, port })
}

// ── Wait for redirect code ────────────────────────────────────────────────────

#[tauri::command]
pub fn gdrive_wait_auth(state: State<'_, DbState>, port: u16) -> Result<String, String> {
    let redirect = format!("http://localhost:{}", port);

    // Start local server with 3-minute timeout
    let listener =
        TcpListener::bind(format!("127.0.0.1:{}", port)).map_err(|e| e.to_string())?;

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        if let Ok((stream, _)) = listener.accept() {
            tx.send(stream).ok();
        }
    });

    let stream = rx.recv_timeout(Duration::from_secs(180))
        .map_err(|_| "Authorization timed out (3 minutes). Please try again.".to_string())?;
    let reader = BufReader::new(&stream);
    let first_line = reader.lines().next().ok_or("no request")?.map_err(|e| e.to_string())?;

    // Parse code from GET /?code=...
    let code = first_line
        .split_whitespace()
        .nth(1)
        .and_then(|path| {
            path.split('?').nth(1).and_then(|qs| {
                qs.split('&').find(|p| p.starts_with("code=")).map(|p| p[5..].to_string())
            })
        })
        .ok_or("no code in redirect")?;

    // Send success page back to browser
    let html = b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
        <html><body style='font-family:sans-serif;padding:40px;background:#1a1a1a;color:#e0e0e0'>\
        <h2 style='color:#4A9EFF'>TaskClaw connected to Google Drive</h2>\
        <p>You can close this tab.</p></body></html>";
    { let mut s = &stream; s.write_all(html).ok(); }

    // Exchange code for tokens
    let body = format!(
        "code={}&client_id={}&client_secret={}&redirect_uri={}&grant_type=authorization_code",
        urlencoding::encode(&code),
        urlencoding::encode(CLIENT_ID),
        urlencoding::encode(CLIENT_SECRET),
        urlencoding::encode(&redirect)
    );

    let resp = reqwest::blocking::Client::new()
        .post(TOKEN_URI)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .map_err(|e| e.to_string())?;
    let resp_json: serde_json::Value = resp.json().map_err(|e| e.to_string())?;

    if let Some(err) = resp_json.get("error") {
        return Err(format!("Token exchange failed: {}", err));
    }

    let access_token = resp_json["access_token"].as_str().ok_or("no access_token")?.to_string();
    let refresh_token = resp_json["refresh_token"].as_str().unwrap_or("").to_string();

    // Store tokens in app_settings
    let conn = state.0.lock().unwrap();
    conn.execute(
        "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2)",
        params!["gdrive_access_token", &access_token],
    ).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2)",
        params!["gdrive_refresh_token", &refresh_token],
    ).map_err(|e| e.to_string())?;

    Ok("Connected to Google Drive".to_string())
}

// ── Token refresh helper ──────────────────────────────────────────────────────

fn refresh_access_token(refresh_token: &str) -> Result<String, String> {
    let body = format!(
        "client_id={}&client_secret={}&refresh_token={}&grant_type=refresh_token",
        urlencoding::encode(CLIENT_ID),
        urlencoding::encode(CLIENT_SECRET),
        urlencoding::encode(refresh_token)
    );
    let resp = reqwest::blocking::Client::new()
        .post(TOKEN_URI)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .map_err(|e| e.to_string())?;
    let json: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
    if let Some(err) = json.get("error") {
        return Err(format!("Token refresh failed: {}", err));
    }
    json["access_token"]
        .as_str()
        .ok_or_else(|| "no access_token in refresh response".to_string())
        .map(|s| s.to_string())
}

fn get_tokens(conn: &rusqlite::Connection) -> Result<(String, String), String> {
    let access: String = conn
        .query_row("SELECT value FROM app_settings WHERE key='gdrive_access_token'", [], |r| r.get(0))
        .map_err(|_| "Not connected to Google Drive. Go to Preferences → Sync to connect.")?;
    let refresh: String = conn
        .query_row("SELECT value FROM app_settings WHERE key='gdrive_refresh_token'", [], |r| r.get(0))
        .unwrap_or_default();
    Ok((access, refresh))
}

fn get_valid_token(conn: &rusqlite::Connection) -> Result<String, String> {
    let (access, refresh) = get_tokens(conn)?;
    if refresh.is_empty() { return Ok(access); }
    // Try refreshing to get a fresh token
    refresh_access_token(&refresh)
}

// ── Check connection status ───────────────────────────────────────────────────

#[tauri::command]
pub fn gdrive_status(state: State<DbState>) -> bool {
    let conn = state.0.lock().unwrap();
    conn.query_row(
        "SELECT 1 FROM app_settings WHERE key='gdrive_refresh_token'",
        [], |_| Ok(1_i32)
    ).is_ok()
}

// ── Disconnect ────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn gdrive_disconnect(state: State<DbState>) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    conn.execute("DELETE FROM app_settings WHERE key IN ('gdrive_access_token','gdrive_refresh_token','gdrive_file_id','gdrive_last_sync')", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ── Build sync payload ────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct SyncPayload {
    version: u32,
    exported_at: String,
    tasks: serde_json::Value,
    flags: serde_json::Value,
    tags: serde_json::Value,
    views: serde_json::Value,
    settings: serde_json::Value,
}

fn build_sync_payload(conn: &rusqlite::Connection) -> Result<SyncPayload, String> {
    fn query_json(conn: &rusqlite::Connection, sql: &str) -> serde_json::Value {
        let mut stmt = conn.prepare(sql).unwrap();
        let cols: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
        let rows: Vec<serde_json::Value> = stmt
            .query_map([], |row| {
                let mut obj = serde_json::Map::new();
                for (i, col) in cols.iter().enumerate() {
                    let val: serde_json::Value = match row.get_ref(i).unwrap() {
                        rusqlite::types::ValueRef::Null    => serde_json::Value::Null,
                        rusqlite::types::ValueRef::Integer(n) => serde_json::Value::Number(n.into()),
                        rusqlite::types::ValueRef::Real(f)    => serde_json::json!(f),
                        rusqlite::types::ValueRef::Text(s)    => serde_json::Value::String(String::from_utf8_lossy(s).into()),
                        rusqlite::types::ValueRef::Blob(_)    => serde_json::Value::Null,
                    };
                    obj.insert(col.clone(), val);
                }
                Ok(serde_json::Value::Object(obj))
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        serde_json::Value::Array(rows)
    }

    // Also pull task_tags
    let tasks_with_tags: serde_json::Value = {
        let tasks = query_json(conn,
            "SELECT id, parent_id, caption, note, position, created_at, updated_at, completed_at,
             start_date, due_date, reminder_at, recurrence_rule, flag_id, starred, color,
             is_folder, is_project, hide_in_views, subtasks_in_order, inherit_dates, custom_format
             FROM tasks");
        let task_tags = query_json(conn, "SELECT task_id, tag_id FROM task_tags");
        // Embed tag_ids in each task
        let mut tasks_arr = tasks.as_array().cloned().unwrap_or_default();
        let tag_map: std::collections::HashMap<String, Vec<String>> =
            task_tags.as_array().unwrap_or(&vec![]).iter().fold(
                std::collections::HashMap::new(),
                |mut m, tt| {
                    let tid = tt["task_id"].as_str().unwrap_or_default().to_string();
                    let gid = tt["tag_id"].as_str().unwrap_or_default().to_string();
                    m.entry(tid).or_default().push(gid);
                    m
                },
            );
        for task in &mut tasks_arr {
            let tid = task["id"].as_str().unwrap_or_default().to_string();
            task["tag_ids"] = serde_json::json!(tag_map.get(&tid).cloned().unwrap_or_default());
        }
        serde_json::Value::Array(tasks_arr)
    };

    Ok(SyncPayload {
        version: 1,
        exported_at: chrono::Utc::now().to_rfc3339(),
        tasks: tasks_with_tags,
        flags: query_json(conn, "SELECT * FROM flags"),
        tags:  query_json(conn, "SELECT * FROM tags"),
        views: query_json(conn, "SELECT * FROM saved_views"),
        settings: query_json(conn, "SELECT key, value FROM app_settings WHERE key NOT LIKE 'gdrive_%'"),
    })
}

// ── Find or create Drive file ─────────────────────────────────────────────────

fn find_or_create_file(
    token: &str,
    conn: &rusqlite::Connection,
) -> Result<String, String> {
    // Check stored file ID
    if let Ok(id) = conn.query_row(
        "SELECT value FROM app_settings WHERE key='gdrive_file_id'",
        [], |r| r.get::<_, String>(0)
    ) {
        // Verify file still exists
        let check = reqwest::blocking::Client::new()
            .get(format!("https://www.googleapis.com/drive/v3/files/{}", id))
            .bearer_auth(token)
            .send()
            .map_err(|e| e.to_string())?;
        if check.status().is_success() {
            return Ok(id);
        }
    }

    // Search for existing file
    let search_url = format!(
        "https://www.googleapis.com/drive/v3/files?q=name%3D%27{}%27+and+trashed%3Dfalse&fields=files(id,name)",
        SYNC_FILE_NAME
    );
    let resp: serde_json::Value = reqwest::blocking::Client::new()
        .get(&search_url)
        .bearer_auth(token)
        .send()
        .map_err(|e| e.to_string())?
        .json()
        .map_err(|e| e.to_string())?;

    if let Some(id) = resp["files"].as_array()
        .and_then(|f| f.first())
        .and_then(|f| f["id"].as_str())
    {
        let file_id = id.to_string();
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key,value) VALUES ('gdrive_file_id',?1)",
            params![&file_id],
        ).ok();
        return Ok(file_id);
    }

    // Create new file
    let meta = serde_json::json!({ "name": SYNC_FILE_NAME, "mimeType": "application/json" });
    let create_resp: serde_json::Value = reqwest::blocking::Client::new()
        .post("https://www.googleapis.com/drive/v3/files")
        .bearer_auth(token)
        .json(&meta)
        .send()
        .map_err(|e| e.to_string())?
        .json()
        .map_err(|e| e.to_string())?;

    let file_id = create_resp["id"].as_str().ok_or("no id in create response")?.to_string();
    conn.execute(
        "INSERT OR REPLACE INTO app_settings (key,value) VALUES ('gdrive_file_id',?1)",
        params![&file_id],
    ).ok();
    Ok(file_id)
}

// ── Push (upload) ─────────────────────────────────────────────────────────────

#[tauri::command]
pub fn gdrive_sync_push(state: State<DbState>) -> Result<String, String> {
    let conn = state.0.lock().unwrap();
    let token = get_valid_token(&conn)?;
    let payload = build_sync_payload(&conn)?;
    let json_str = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    let file_id = find_or_create_file(&token, &conn)?;

    // Upload content
    let upload_url = format!(
        "https://www.googleapis.com/upload/drive/v3/files/{}?uploadType=media",
        file_id
    );
    let resp = reqwest::blocking::Client::new()
        .patch(&upload_url)
        .bearer_auth(&token)
        .header("Content-Type", "application/json")
        .body(json_str)
        .send()
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let body = resp.text().unwrap_or_default();
        return Err(format!("Upload failed: {}", body));
    }

    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR REPLACE INTO app_settings (key,value) VALUES ('gdrive_last_sync',?1)",
        params![&now],
    ).ok();

    Ok(format!("Synced at {}", now))
}

// ── Pull (download + restore) ─────────────────────────────────────────────────

#[tauri::command]
pub fn gdrive_sync_pull(state: State<DbState>) -> Result<String, String> {
    let conn = state.0.lock().unwrap();
    let token = get_valid_token(&conn)?;
    let file_id = find_or_create_file(&token, &conn)?;

    // Download content
    let download_url = format!(
        "https://www.googleapis.com/drive/v3/files/{}?alt=media",
        file_id
    );
    let resp = reqwest::blocking::Client::new()
        .get(&download_url)
        .bearer_auth(&token)
        .send()
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let body = resp.text().unwrap_or_default();
        return Err(format!("Download failed: {}", body));
    }

    let payload: SyncPayload = resp.json().map_err(|e| format!("Invalid sync file: {}", e))?;

    // Restore — replace all data
    restore_payload(&conn, &payload)?;

    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR REPLACE INTO app_settings (key,value) VALUES ('gdrive_last_sync',?1)",
        params![&now],
    ).ok();

    Ok(format!("Restored from {}", payload.exported_at))
}

fn restore_payload(conn: &rusqlite::Connection, payload: &SyncPayload) -> Result<(), String> {
    // Clear existing data (preserve app_settings)
    conn.execute_batch("
        DELETE FROM task_tags;
        DELETE FROM email_links;
        DELETE FROM tasks;
        DELETE FROM flags;
        DELETE FROM tags;
        DELETE FROM saved_views;
    ").map_err(|e| e.to_string())?;

    // Restore flags
    if let Some(flags) = payload.flags.as_array() {
        for f in flags {
            conn.execute(
                "INSERT OR IGNORE INTO flags (id, name, color, position) VALUES (?1,?2,?3,?4)",
                params![
                    f["id"].as_str().unwrap_or(""),
                    f["name"].as_str().unwrap_or(""),
                    f["color"].as_str().unwrap_or("#888"),
                    f["position"].as_i64().unwrap_or(0),
                ],
            ).map_err(|e| e.to_string())?;
        }
    }

    // Restore tags
    if let Some(tags) = payload.tags.as_array() {
        for t in tags {
            conn.execute(
                "INSERT OR IGNORE INTO tags (id, name, color) VALUES (?1,?2,?3)",
                params![
                    t["id"].as_str().unwrap_or(""),
                    t["name"].as_str().unwrap_or(""),
                    t["color"].as_str().unwrap_or("#888"),
                ],
            ).map_err(|e| e.to_string())?;
        }
    }

    // Restore tasks
    if let Some(tasks) = payload.tasks.as_array() {
        for t in tasks {
            conn.execute(
                "INSERT OR IGNORE INTO tasks
                 (id, parent_id, caption, note, position, created_at, updated_at, completed_at,
                  start_date, due_date, reminder_at, recurrence_rule, flag_id, starred, color,
                  is_folder, is_project, hide_in_views, subtasks_in_order, inherit_dates, custom_format)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21)",
                params![
                    t["id"].as_str().unwrap_or(""),
                    t["parent_id"].as_str(),
                    t["caption"].as_str().unwrap_or(""),
                    t["note"].as_str().unwrap_or(""),
                    t["position"].as_f64().unwrap_or(0.0),
                    t["created_at"].as_str().unwrap_or(""),
                    t["updated_at"].as_str().unwrap_or(""),
                    t["completed_at"].as_str(),
                    t["start_date"].as_str(),
                    t["due_date"].as_str(),
                    t["reminder_at"].as_str(),
                    t["recurrence_rule"].as_str(),
                    t["flag_id"].as_str(),
                    t["starred"].as_bool().unwrap_or(false) as i32,
                    t["color"].as_str(),
                    t["is_folder"].as_bool().unwrap_or(false) as i32,
                    t["is_project"].as_bool().unwrap_or(false) as i32,
                    t["hide_in_views"].as_bool().unwrap_or(false) as i32,
                    t["subtasks_in_order"].as_bool().unwrap_or(false) as i32,
                    t["inherit_dates"].as_bool().unwrap_or(false) as i32,
                    t["custom_format"].as_str(),
                ],
            ).map_err(|e| e.to_string())?;

            // Restore task_tags
            if let Some(tag_ids) = t["tag_ids"].as_array() {
                let task_id = t["id"].as_str().unwrap_or("");
                for tag_id in tag_ids {
                    if let Some(tid) = tag_id.as_str() {
                        conn.execute(
                            "INSERT OR IGNORE INTO task_tags (task_id, tag_id) VALUES (?1,?2)",
                            params![task_id, tid],
                        ).ok();
                    }
                }
            }
        }
    }

    // Restore views
    if let Some(views) = payload.views.as_array() {
        for v in views {
            conn.execute(
                "INSERT OR IGNORE INTO saved_views
                 (id, name, show_completed, group_by, sort_by, sort_dir, visible_fields, filter_json, position)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![
                    v["id"].as_str().unwrap_or(""),
                    v["name"].as_str().unwrap_or(""),
                    v["show_completed"].as_bool().unwrap_or(false) as i32,
                    v["group_by"].as_str().unwrap_or("none"),
                    v["sort_by"].as_str().unwrap_or("position"),
                    v["sort_dir"].as_str().unwrap_or("asc"),
                    v["visible_fields"].as_str().unwrap_or("[]"),
                    v["filter_json"].as_str().unwrap_or("{}"),
                    v["position"].as_i64().unwrap_or(0),
                ],
            ).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

// ── Get last sync time ────────────────────────────────────────────────────────

#[tauri::command]
pub fn gdrive_last_sync(state: State<DbState>) -> Option<String> {
    let conn = state.0.lock().unwrap();
    conn.query_row(
        "SELECT value FROM app_settings WHERE key='gdrive_last_sync'",
        [], |r| r.get(0)
    ).ok()
}
