/// Google Drive sync — uploads/downloads taskclaw.db as Drive appdata file.
/// Uses the installed-app OAuth2 loopback flow (RFC 8252).
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;

// ── Credentials (injected at build time via env vars) ────────────────────────
const CLIENT_ID: &str = env!("GDRIVE_CLIENT_ID");
const CLIENT_SECRET: &str = env!("GDRIVE_CLIENT_SECRET");
const SCOPE: &str = "https://www.googleapis.com/auth/drive.appdata";
const TOKEN_FILENAME: &str = "gdrive_token.json";
const DRIVE_FILENAME: &str = "taskclaw.db";

// ── Token storage ─────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToken {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64, // unix timestamp
}

fn token_path() -> PathBuf {
    let mut p = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("taskclaw");
    p.push(TOKEN_FILENAME);
    p
}

pub fn load_token() -> Option<StoredToken> {
    let path = token_path();
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save_token(token: &StoredToken) {
    let path = token_path();
    if let Ok(data) = serde_json::to_string_pretty(token) {
        std::fs::write(path, data).ok();
    }
}

// ── OAuth loopback flow ───────────────────────────────────────────────────────
/// Binds a random local port, returns (port, listener).
fn bind_loopback() -> std::io::Result<(u16, TcpListener)> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    Ok((port, listener))
}

/// Extracts `code` query param from the raw HTTP GET line.
fn extract_code(request_line: &str) -> Option<String> {
    // GET /?code=XYZ&... HTTP/1.1
    let path = request_line.split_whitespace().nth(1)?;
    url::Url::parse(&format!("http://localhost{}", path)).ok()?
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.into_owned())
}

/// Opens browser for OAuth consent; blocks until callback received; returns auth code.
pub fn run_oauth_flow() -> Result<String, String> {
    let (port, listener) = bind_loopback().map_err(|e| e.to_string())?;
    let redirect_uri = format!("http://127.0.0.1:{}", port);

    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent",
        CLIENT_ID,
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(SCOPE),
    );

    // Open browser
    #[cfg(target_os = "windows")]
    std::process::Command::new("cmd").args(["/c", "start", &auth_url]).spawn().ok();
    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(&auth_url).spawn().ok();
    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open").arg(&auth_url).spawn().ok();

    // Wait for callback
    listener.set_nonblocking(false).ok();
    let (mut stream, _) = listener.accept().map_err(|e| e.to_string())?;
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).map_err(|e| e.to_string())?;
    let request = String::from_utf8_lossy(&buf[..n]);
    let first_line = request.lines().next().unwrap_or("");

    // Respond to browser
    let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
        <html><body style='font-family:sans-serif;background:#141414;color:#d4d4d4;padding:40px'>\
        <h2 style='color:#4A9EFF'>TaskClaw</h2><p>Auth complete. You can close this tab.</p></body></html>");

    extract_code(first_line).ok_or_else(|| "No auth code in callback".to_string())
}

// ── Token exchange ────────────────────────────────────────────────────────────
#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
}

pub async fn exchange_code(code: &str, port: u16) -> Result<StoredToken, String> {
    let redirect_uri = format!("http://127.0.0.1:{}", port);
    let client = Client::new();
    let res: TokenResponse = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", code),
            ("client_id", CLIENT_ID),
            ("client_secret", CLIENT_SECRET),
            ("redirect_uri", &redirect_uri),
            ("grant_type", "authorization_code"),
        ])
        .send().await.map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())?;

    let expires_at = chrono::Utc::now().timestamp() + res.expires_in - 60;
    Ok(StoredToken {
        access_token: res.access_token,
        refresh_token: res.refresh_token.unwrap_or_default(),
        expires_at,
    })
}

pub async fn refresh_access_token(refresh_token: &str) -> Result<StoredToken, String> {
    let client = Client::new();
    let res: TokenResponse = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("refresh_token", refresh_token),
            ("client_id", CLIENT_ID),
            ("client_secret", CLIENT_SECRET),
            ("grant_type", "refresh_token"),
        ])
        .send().await.map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())?;

    let expires_at = chrono::Utc::now().timestamp() + res.expires_in - 60;
    Ok(StoredToken {
        access_token: res.access_token,
        refresh_token: refresh_token.to_string(),
        expires_at,
    })
}

/// Returns a valid access token, refreshing if needed.
pub async fn get_access_token() -> Result<String, String> {
    let token = load_token().ok_or("Not authenticated. Run gdrive_connect first.")?;
    if chrono::Utc::now().timestamp() < token.expires_at {
        return Ok(token.access_token);
    }
    let refreshed = refresh_access_token(&token.refresh_token).await?;
    save_token(&refreshed);
    Ok(refreshed.access_token)
}

// ── Drive API ─────────────────────────────────────────────────────────────────
#[derive(Deserialize)]
struct DriveFile {
    id: String,
    #[serde(rename = "modifiedTime")]
    modified_time: Option<String>,
}

#[derive(Deserialize)]
struct DriveFileList {
    files: Vec<DriveFile>,
}

async fn find_remote_file(access_token: &str) -> Result<Option<DriveFile>, String> {
    let client = Client::new();
    let res: DriveFileList = client
        .get("https://www.googleapis.com/drive/v3/files")
        .bearer_auth(access_token)
        .query(&[
            ("spaces", "appDataFolder"),
            ("fields", "files(id,modifiedTime)"),
            ("q", &format!("name='{}'", DRIVE_FILENAME)),
        ])
        .send().await.map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())?;

    Ok(res.files.into_iter().next())
}

/// Upload the local DB file. Creates or overwrites the remote copy.
pub async fn upload_db(db_path: &PathBuf) -> Result<String, String> {
    let access_token = get_access_token().await?;
    let data = std::fs::read(db_path).map_err(|e| e.to_string())?;
    let client = Client::new();

    let existing = find_remote_file(&access_token).await?;

    let url = if let Some(f) = &existing {
        // Update existing file
        format!("https://www.googleapis.com/upload/drive/v3/files/{}?uploadType=multipart", f.id)
    } else {
        "https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart".to_string()
    };

    let meta = if existing.is_some() {
        serde_json::json!({})
    } else {
        serde_json::json!({
            "name": DRIVE_FILENAME,
            "parents": ["appDataFolder"]
        })
    };

    let method = if existing.is_some() {
        reqwest::Method::PATCH
    } else {
        reqwest::Method::POST
    };

    let boundary = "taskclaw_boundary_xk3j9f";
    let mut body = Vec::new();
    write!(body, "--{}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{}\r\n", boundary, meta).ok();
    write!(body, "--{}\r\nContent-Type: application/octet-stream\r\n\r\n", boundary).ok();
    body.extend_from_slice(&data);
    write!(body, "\r\n--{}--\r\n", boundary).ok();

    client.request(method, &url)
        .bearer_auth(&access_token)
        .header("Content-Type", format!("multipart/related; boundary={}", boundary))
        .body(body)
        .send().await.map_err(|e| e.to_string())?;

    Ok(chrono::Utc::now().to_rfc3339())
}

/// Download remote DB. Returns false if remote is older than local (skip).
pub async fn download_db(db_path: &PathBuf) -> Result<bool, String> {
    let access_token = get_access_token().await?;
    let remote = find_remote_file(&access_token).await?;

    let Some(file) = remote else {
        return Ok(false); // nothing on Drive yet
    };

    // Compare timestamps — skip if local is newer
    if db_path.exists() {
        if let Some(remote_time_str) = &file.modified_time {
            if let Ok(remote_ts) = chrono::DateTime::parse_from_rfc3339(remote_time_str) {
                if let Ok(local_meta) = std::fs::metadata(db_path) {
                    if let Ok(local_modified) = local_meta.modified() {
                        let local_ts = chrono::DateTime::<chrono::Utc>::from(local_modified);
                        if local_ts > remote_ts.with_timezone(&chrono::Utc) {
                            return Ok(false); // local is newer, don't overwrite
                        }
                    }
                }
            }
        }
    }

    let client = Client::new();
    let bytes = client
        .get(format!("https://www.googleapis.com/drive/v3/files/{}?alt=media", file.id))
        .bearer_auth(&access_token)
        .send().await.map_err(|e| e.to_string())?
        .bytes().await.map_err(|e| e.to_string())?;

    // Write to a temp path first, then rename (atomic on most OSes)
    let tmp = db_path.with_extension("tmp");
    std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, db_path).map_err(|e| e.to_string())?;

    Ok(true)
}
