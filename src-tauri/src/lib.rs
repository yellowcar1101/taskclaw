mod types;
mod db;
mod gdrive;
mod commands;

use commands::tasks::DbState;
use commands::tasks::*;
use commands::contexts::*;
use commands::sync::*;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let conn = db::open().expect("Failed to open database");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(DbState(Mutex::new(conn)))
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
            // Contexts & links
            get_contexts,
            create_context,
            delete_context,
            add_email_link,
            // GDrive sync
            gdrive_auth_status,
            gdrive_connect,
            gdrive_upload,
            gdrive_download,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
