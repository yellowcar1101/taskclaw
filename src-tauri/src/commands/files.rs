use tauri::State;
use std::sync::Mutex;
use std::path::PathBuf;
use crate::db;
use crate::commands::tasks::DbState;

/// Tracks the path of the currently open database file.
pub struct DbPath(pub Mutex<PathBuf>);

// ── Commands ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn file_current_path(path: State<'_, DbPath>) -> String {
    path.0.lock().unwrap().to_string_lossy().to_string()
}

/// Create a new empty database at `new_path` and switch to it.
#[tauri::command]
pub fn file_new(
    new_path: String,
    db: State<'_, DbState>,
    path: State<'_, DbPath>,
) -> Result<(), String> {
    let dest = PathBuf::from(&new_path);
    // Ensure parent exists
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let new_conn = db::open_at(&dest).map_err(|e| e.to_string())?;
    *db.0.lock().unwrap() = new_conn;
    *path.0.lock().unwrap() = dest;
    Ok(())
}

/// Open an existing database file and switch to it.
#[tauri::command]
pub fn file_open(
    new_path: String,
    db: State<'_, DbState>,
    path: State<'_, DbPath>,
) -> Result<(), String> {
    let dest = PathBuf::from(&new_path);
    if !dest.exists() {
        return Err(format!("File not found: {}", new_path));
    }
    let new_conn = db::open_at(&dest).map_err(|e| e.to_string())?;
    *db.0.lock().unwrap() = new_conn;
    *path.0.lock().unwrap() = dest;
    Ok(())
}

/// Checkpoint and copy the current DB to `dest`, then switch to the copy.
#[tauri::command]
pub fn file_save_as(
    dest: String,
    db: State<'_, DbState>,
    path: State<'_, DbPath>,
) -> Result<(), String> {
    let dest_path = PathBuf::from(&dest);

    // Step 1: checkpoint so the main file is fully consistent
    db.0.lock().unwrap()
        .execute_batch("PRAGMA wal_checkpoint(FULL);")
        .map_err(|e| e.to_string())?;

    // Step 2: get current path (without holding db lock)
    let current = path.0.lock().unwrap().clone();

    // Step 3: copy the file
    std::fs::copy(&current, &dest_path).map_err(|e| e.to_string())?;

    // Step 4: switch to the new copy
    let new_conn = db::open_at(&dest_path).map_err(|e| e.to_string())?;
    *db.0.lock().unwrap() = new_conn;
    *path.0.lock().unwrap() = dest_path;
    Ok(())
}
