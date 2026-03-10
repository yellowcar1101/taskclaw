mod types;
mod db;
mod gdrive;
mod commands;

use commands::tasks::DbState;
use commands::tasks::*;
use commands::flags::*;
use commands::sync::*;
use std::sync::Mutex;

fn startup_log(msg: &str) {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let log_path = exe_dir.join("startup.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&log_path) {
        use std::io::Write;
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        writeln!(f, "[{}] {}", ts, msg).ok();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    startup_log("run() called");

    let conn_opt = match db::open() {
        Ok(c)  => { startup_log("DB opened OK (unencrypted)"); Some(c) }
        Err(e) => { startup_log(&format!("DB open failed (may be encrypted): {}", e)); None }
    };

    startup_log("building Tauri app");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(DbState(Mutex::new(conn_opt)))
        .invoke_handler(tauri::generate_handler![
            // Tasks
            get_tasks,
            get_all_tasks_flat,
            create_task,
            update_task,
            delete_task,
            complete_task,
            move_task,
            reorder_tasks,
            // Flags & tags
            get_flags,
            create_flag,
            update_flag,
            delete_flag,
            get_tags,
            create_tag,
            delete_tag,
            // Email links
            add_email_link,
            delete_email_link,
            // Views
            get_views,
            create_view,
            update_view,
            delete_view,
            // GDrive sync
            gdrive_auth_status,
            gdrive_connect,
            gdrive_upload,
            gdrive_download,
            // Encryption
            check_db_encrypted,
            is_db_locked,
            unlock_db,
            set_db_password,
        ])
        .setup(|_app| {
            startup_log("Tauri setup() called — window about to open");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    startup_log("run() returned (unexpected)");
}
