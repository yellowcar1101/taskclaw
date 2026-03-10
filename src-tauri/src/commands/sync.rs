use tauri::State;
use crate::commands::tasks::DbState;
use crate::db::db_path;
use crate::gdrive;
use crate::types::SyncResult;
use chrono::Utc;

#[tauri::command]
pub async fn gdrive_auth_status() -> bool {
    gdrive::load_token()
        .map(|t| chrono::Utc::now().timestamp() < t.expires_at || !t.refresh_token.is_empty())
        .unwrap_or(false)
}

#[tauri::command]
pub async fn gdrive_connect() -> Result<SyncResult, String> {
    // Run OAuth flow on a blocking thread (it blocks on TCP accept)
    let result = tokio::task::spawn_blocking(|| -> Result<(String, u16), String> {
        let (port, listener) = {
            let l = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
            let port = l.local_addr().map_err(|e| e.to_string())?.port();
            (port, l)
        };

        // Build auth URL and open browser
        let redirect_uri = format!("http://127.0.0.1:{}", port);
        let auth_url = format!(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent",
            env!("GDRIVE_CLIENT_ID"),
            urlencoding::encode(&redirect_uri),
            urlencoding::encode("https://www.googleapis.com/auth/drive.appdata"),
        );

        #[cfg(target_os = "windows")]
        std::process::Command::new("cmd").args(["/c", "start", &auth_url]).spawn().ok();
        #[cfg(target_os = "linux")]
        std::process::Command::new("xdg-open").arg(&auth_url).spawn().ok();
        #[cfg(target_os = "macos")]
        std::process::Command::new("open").arg(&auth_url).spawn().ok();

        // Wait for redirect
        let (mut stream, _) = listener.accept().map_err(|e| e.to_string())?;
        use std::io::{Read, Write};
        let mut buf = [0u8; 4096];
        let n = stream.read(&mut buf).map_err(|e| e.to_string())?;
        let request = String::from_utf8_lossy(&buf[..n]);
        let first_line = request.lines().next().unwrap_or("").to_string();
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
            <html><body style='font-family:sans-serif;background:#141414;color:#d4d4d4;padding:40px'>\
            <h2 style='color:#4A9EFF'>TaskClaw</h2><p>Connected to Google Drive. You can close this tab.</p></body></html>");

        // Extract code
        let path = first_line.split_whitespace().nth(1).unwrap_or("/");
        let code = url::Url::parse(&format!("http://localhost{}", path))
            .ok()
            .and_then(|u| u.query_pairs().find(|(k, _)| k == "code").map(|(_, v)| v.into_owned()))
            .ok_or("No auth code received")?;

        Ok((code, port))
    }).await.map_err(|e| e.to_string())??;

    let (code, port) = result;
    let token = gdrive::exchange_code(&code, port).await?;
    gdrive::save_token(&token);

    Ok(SyncResult {
        success: true,
        message: "Google Drive connected.".into(),
        synced_at: Some(Utc::now().to_rfc3339()),
    })
}

#[tauri::command]
pub async fn gdrive_upload(_state: State<'_, DbState>) -> Result<SyncResult, String> {
    // Flush WAL before upload
    {
        let conn = _state.0.lock().unwrap();
        conn.execute_batch("PRAGMA wal_checkpoint(FULL);").ok();
    }
    let synced_at = gdrive::upload_db(&db_path()).await?;
    Ok(SyncResult {
        success: true,
        message: "Uploaded to Google Drive.".into(),
        synced_at: Some(synced_at),
    })
}

#[tauri::command]
pub async fn gdrive_download(state: State<'_, DbState>) -> Result<SyncResult, String> {
    let downloaded = gdrive::download_db(&db_path()).await?;
    if downloaded {
        // Re-open the DB connection so the new data is loaded
        // (Tauri manages the state — signal the frontend to reload)
        let _ = state; // frontend will call loadAll() on success
        Ok(SyncResult {
            success: true,
            message: "Downloaded from Google Drive. Reload your task list.".into(),
            synced_at: Some(Utc::now().to_rfc3339()),
        })
    } else {
        Ok(SyncResult {
            success: true,
            message: "Local data is up to date — nothing downloaded.".into(),
            synced_at: None,
        })
    }
}
