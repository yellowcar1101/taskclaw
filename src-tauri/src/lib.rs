mod types;
mod db;
mod commands;

use commands::tasks::DbState;
use commands::tasks::*;
use commands::flags::*;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let conn = db::open().expect("Failed to open database");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
