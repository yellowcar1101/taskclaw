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
    path.0.lock().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()
}

fn validate_db_extension(p: &PathBuf) -> Result<(), String> {
    match p.extension().and_then(|e| e.to_str()) {
        Some("db") => Ok(()),
        _ => Err("only .db files are supported".into()),
    }
}

/// Create a new empty database at `new_path` and switch to it.
#[tauri::command]
pub fn file_new(
    new_path: String,
    db: State<'_, DbState>,
    path: State<'_, DbPath>,
) -> Result<(), String> {
    let dest = PathBuf::from(&new_path);
    validate_db_extension(&dest)?;
    // Ensure parent exists
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let new_conn = db::open_at(&dest).map_err(|e| e.to_string())?;
    *db.0.lock().map_err(|e| e.to_string())? = new_conn;
    *path.0.lock().map_err(|e| e.to_string())? = dest;
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
    validate_db_extension(&dest)?;
    if !dest.exists() {
        return Err(format!("File not found: {}", new_path));
    }
    let new_conn = db::open_at(&dest).map_err(|e| e.to_string())?;
    *db.0.lock().map_err(|e| e.to_string())? = new_conn;
    *path.0.lock().map_err(|e| e.to_string())? = dest;
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
    validate_db_extension(&dest_path)?;

    // Step 1: checkpoint so the main file is fully consistent
    db.0.lock().map_err(|e| e.to_string())?
        .execute_batch("PRAGMA wal_checkpoint(FULL);")
        .map_err(|e| e.to_string())?;

    // Step 2: get current path (without holding db lock)
    let current = path.0.lock().map_err(|e| e.to_string())?.clone();

    // Step 3: copy the file
    std::fs::copy(&current, &dest_path).map_err(|e| e.to_string())?;

    // Step 4: switch to the new copy
    let new_conn = db::open_at(&dest_path).map_err(|e| e.to_string())?;
    *db.0.lock().map_err(|e| e.to_string())? = new_conn;
    *path.0.lock().map_err(|e| e.to_string())? = dest_path;
    Ok(())
}
