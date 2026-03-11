use tauri::{AppHandle, Manager};
use serde::{Deserialize, Serialize};
use crate::db;

// ── Startup config ─────────────────────────────────────────────────────────────
// Stored in Data/taskclaw-config.json; read before the DB opens so it can
// control plugin initialisation in lib.rs.

#[derive(Serialize, Deserialize, Clone)]
pub struct StartupConfig {
    #[serde(default)]
    pub remember_position: bool,
    #[serde(default = "default_true")]
    pub single_instance: bool,
}

fn default_true() -> bool { true }

impl StartupConfig {
    pub fn defaults() -> Self {
        Self { remember_position: false, single_instance: true }
    }
}

pub fn read_startup_config() -> StartupConfig {
    let path = db::data_dir().join("taskclaw-config.json");
    match std::fs::read_to_string(&path) {
        Ok(s) if !s.is_empty() => {
            serde_json::from_str(&s).unwrap_or_else(|_| StartupConfig::defaults())
        }
        _ => StartupConfig::defaults(),
    }
}

// ── Window state ───────────────────────────────────────────────────────────────
// Stores outer position + inner size in Data/window-state.json.

#[derive(Serialize, Deserialize, Default)]
struct WindowState {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

/// Called from setup() in lib.rs when remember_position is enabled.
pub fn apply_window_state(handle: AppHandle) {
    let path = db::data_dir().join("window-state.json");
    if let Ok(s) = std::fs::read_to_string(&path) {
        if let Ok(state) = serde_json::from_str::<WindowState>(&s) {
            if let Some(win) = handle.get_webview_window("main") {
                let _ = win.set_size(tauri::Size::Physical(tauri::PhysicalSize {
                    width: state.width,
                    height: state.height,
                }));
                let _ = win.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                    x: state.x,
                    y: state.y,
                }));
            }
        }
    }
}

/// Invoked from the frontend on window close (beforeunload).
#[tauri::command]
pub fn save_window_state(app: AppHandle) -> Result<(), String> {
    let win = match app.get_webview_window("main") {
        Some(w) => w,
        None => return Ok(()),
    };
    let pos  = win.outer_position().map_err(|e| e.to_string())?;
    let size = win.inner_size().map_err(|e| e.to_string())?;
    let state = WindowState { x: pos.x, y: pos.y, width: size.width, height: size.height };
    let path = db::data_dir().join("window-state.json");
    std::fs::write(path, serde_json::to_string(&state).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_startup_config() -> StartupConfig {
    read_startup_config()
}

#[tauri::command]
pub fn save_startup_config(config: StartupConfig) -> Result<(), String> {
    let path = db::data_dir().join("taskclaw-config.json");
    let s = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    std::fs::write(path, s).map_err(|e| e.to_string())
}

// ── Reminder window ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn show_reminder_window(app: AppHandle) {
    if let Some(win) = app.get_webview_window("reminder") {
        win.show().ok();
        win.set_focus().ok();
        return;
    }
    let _ = tauri::WebviewWindowBuilder::new(
        &app, "reminder", tauri::WebviewUrl::App("reminder".into()),
    )
    .title("TaskClaw — Reminder")
    .inner_size(360.0, 460.0)
    .always_on_top(true)
    .resizable(false)
    .build();
}

#[tauri::command]
pub fn hide_reminder_window(app: AppHandle) {
    if let Some(win) = app.get_webview_window("reminder") {
        win.hide().ok();
    }
}
