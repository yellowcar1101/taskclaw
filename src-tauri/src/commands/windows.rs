use tauri::AppHandle;

/// Show (or create-then-show) the reminder window.
/// Called by the main window when reminders are due.
#[tauri::command]
pub fn show_reminder_window(app: AppHandle) {
    if let Some(win) = app.get_webview_window("reminder") {
        win.show().ok();
        win.set_focus().ok();
        return;
    }
    // First time: create it
    let _ = tauri::WebviewWindowBuilder::new(
        &app,
        "reminder",
        tauri::WebviewUrl::App("reminder".into()),
    )
    .title("TaskClaw — Reminder")
    .inner_size(360.0, 460.0)
    .always_on_top(true)
    .resizable(false)
    .build();
}

/// Hide the reminder window (called from the reminder window itself when all cards are gone).
#[tauri::command]
pub fn hide_reminder_window(app: AppHandle) {
    if let Some(win) = app.get_webview_window("reminder") {
        win.hide().ok();
    }
}
