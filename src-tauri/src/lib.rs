mod types;
mod db;
mod commands;

use commands::tasks::DbState;
use commands::tasks::*;
use commands::contexts::*;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let conn = db::open().expect("Failed to open database");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(DbState(Mutex::new(conn)))
        .invoke_handler(tauri::generate_handler![
            get_tasks,
            get_all_tasks_flat,
            create_task,
            update_task,
            delete_task,
            complete_task,
            move_task,
            reorder_tasks,
            get_contexts,
            create_context,
            delete_context,
            add_email_link,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
