mod types;
mod db;
mod commands;

use commands::tasks::DbState;
use commands::tasks::*;
use commands::flags::*;
use commands::sync::*;
use commands::webapi::{self, *};
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let conn = db::open().expect("Failed to open database");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|_app| {
            webapi::autostart_if_enabled();
            Ok(())
        })
        .manage(DbState(Mutex::new(conn)))
        .invoke_handler(tauri::generate_handler![
            // tasks
            get_tasks,
            get_all_tasks_flat,
            create_task,
            update_task,
            delete_task,
            delete_task_recursive,
            complete_task,
            complete_branch,
            move_task,
            reorder_tasks,
            duplicate_task,
            sort_subtasks,
            skip_occurrence,
            // flags
            get_flags,
            create_flag,
            update_flag,
            delete_flag,
            reorder_flags,
            // tags
            get_tags,
            create_tag,
            update_tag,
            delete_tag,
            // views
            get_views,
            create_view,
            update_view,
            delete_view,
            reorder_views,
            // email links
            add_email_link,
            delete_email_link,
            // settings
            get_setting,
            set_setting,
            get_all_settings,
            // gdrive sync
            gdrive_auth_url,
            gdrive_wait_auth,
            gdrive_status,
            gdrive_disconnect,
            gdrive_sync_push,
            gdrive_sync_pull,
            gdrive_last_sync,
            gdrive_set_credentials,
            gdrive_has_custom_credentials,
            // folder sync
            set_sync_folder,
            get_sync_folder,
            folder_sync_push,
            folder_sync_pull,
            folder_last_sync,
            // web api
            webapi_start,
            webapi_set_token,
            webapi_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
