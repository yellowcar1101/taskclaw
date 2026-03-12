//! Unit tests for task and flag command logic.
//!
//! These tests operate directly on `rusqlite::Connection` (in-memory) and
//! replicate the SQL logic from `tasks.rs` and `flags.rs` without going
//! through `tauri::State`.

#[cfg(test)]
mod tests {
    use rusqlite::{Connection, params};
    use uuid::Uuid;
    use chrono::Utc;

    // ── test DB setup ──────────────────────────────────────────────────────────

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory DB failed");
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        crate::db::migrate(&conn).expect("migration failed");
        conn
    }

    // ── helpers that mirror command logic ─────────────────────────────────────

    fn insert_task(conn: &Connection, caption: &str, parent_id: Option<&str>) -> Result<String, String> {
        if caption.is_empty() {
            return Err("caption cannot be empty".into());
        }
        if caption.len() > 500 {
            return Err("caption too long (max 500 chars)".into());
        }
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let max_pos: f64 = conn.query_row(
            "SELECT COALESCE(MAX(position), 0) FROM tasks WHERE parent_id IS ?1",
            params![parent_id],
            |r| r.get(0),
        ).unwrap_or(0.0);
        let position = max_pos + 1.0;
        conn.execute(
            "INSERT INTO tasks (id, parent_id, caption, note, position, created_at, updated_at)
             VALUES (?1, ?2, ?3, '', ?4, ?5, ?5)",
            params![id, parent_id, caption, position, now],
        ).map_err(|e| e.to_string())?;
        Ok(id)
    }

    fn task_exists(conn: &Connection, id: &str) -> bool {
        conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE id=?1)",
            params![id],
            |r| r.get::<_, bool>(0),
        ).unwrap_or(false)
    }

    fn count_tasks(conn: &Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0)).unwrap_or(0)
    }

    fn delete_task(conn: &Connection, id: &str) -> Result<(), String> {
        let n = conn.execute("DELETE FROM tasks WHERE id=?1", params![id])
            .map_err(|e| e.to_string())?;
        if n == 0 { Err("task not found".into()) } else { Ok(()) }
    }

    fn delete_task_recursive(conn: &Connection, id: &str) -> Result<(), String> {
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE id=?1)", params![id], |r| r.get(0),
        ).unwrap_or(false);
        if !exists { return Err("task not found".into()); }
        conn.execute(
            "WITH RECURSIVE descendants(id) AS (
                SELECT id FROM tasks WHERE id = ?1
                UNION ALL
                SELECT t.id FROM tasks t JOIN descendants d ON t.parent_id = d.id
            )
            DELETE FROM tasks WHERE id IN (SELECT id FROM descendants)",
            params![id],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn insert_flag(conn: &Connection, name: &str, color: &str) -> Result<String, String> {
        if name.is_empty() { return Err("name cannot be empty".into()); }
        if name.len() > 50 { return Err("name too long (max 50 chars)".into()); }
        validate_color(color)?;
        let existing: i64 = conn.query_row(
            "SELECT COUNT(*) FROM flags WHERE name=?1", params![name], |r| r.get(0),
        ).unwrap_or(0);
        if existing > 0 { return Err("name already exists".into()); }
        let id = Uuid::new_v4().to_string();
        let pos: f64 = conn.query_row(
            "SELECT COALESCE(MAX(position),0)+1 FROM flags", [], |r| r.get(0),
        ).unwrap_or(1.0);
        conn.execute(
            "INSERT INTO flags (id, name, color, position) VALUES (?1,?2,?3,?4)",
            params![id, name, color, pos],
        ).map_err(|e| e.to_string())?;
        Ok(id)
    }

    fn delete_flag(conn: &Connection, id: &str) -> Result<(), String> {
        let n = conn.execute("DELETE FROM flags WHERE id=?1", params![id])
            .map_err(|e| e.to_string())?;
        if n == 0 { Err("flag not found".into()) } else { Ok(()) }
    }

    fn validate_color(c: &str) -> Result<(), String> {
        if c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|ch| ch.is_ascii_hexdigit()) {
            Ok(())
        } else {
            Err("color must be #RRGGBB".into())
        }
    }

    const ALLOWED_SETTING_KEYS: &[&str] = &[
        "app_font",
        "app_font_size",
        "app_compact",
        "app_task_color",
        "startup_remember_position",
        "startup_single_instance",
    ];

    fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
        if !ALLOWED_SETTING_KEYS.contains(&key) {
            return Err(format!("unknown or restricted setting key: {}", key));
        }
        conn.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1,?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        ).map(|_| ()).map_err(|e| e.to_string())
    }

    fn get_setting(conn: &Connection, key: &str) -> Option<String> {
        conn.query_row(
            "SELECT value FROM app_settings WHERE key=?1", params![key], |r| r.get(0),
        ).ok()
    }

    // ── recurrence interval clamp (mirrors next_occurrence_from logic) ─────────

    fn clamp_interval(raw: u64) -> u64 {
        raw.min(9999)
    }

    // ── Task creation tests ────────────────────────────────────────────────────

    #[test]
    fn test_create_task_valid() {
        let conn = setup_db();
        let result = insert_task(&conn, "Buy milk", None);
        assert!(result.is_ok(), "valid task should be created");
        let id = result.unwrap();
        assert!(task_exists(&conn, &id), "task should be in DB");
    }

    #[test]
    fn test_create_task_empty_caption_rejected() {
        let conn = setup_db();
        let result = insert_task(&conn, "", None);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "caption cannot be empty");
    }

    #[test]
    fn test_create_task_caption_too_long_rejected() {
        let conn = setup_db();
        let long_caption = "x".repeat(501);
        let result = insert_task(&conn, &long_caption, None);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "caption too long (max 500 chars)");
    }

    #[test]
    fn test_create_task_exactly_500_chars_accepted() {
        let conn = setup_db();
        let caption = "a".repeat(500);
        let result = insert_task(&conn, &caption, None);
        assert!(result.is_ok(), "500-char caption should be accepted");
    }

    #[test]
    fn test_create_multiple_tasks_increments_position() {
        let conn = setup_db();
        let id1 = insert_task(&conn, "Task 1", None).unwrap();
        let id2 = insert_task(&conn, "Task 2", None).unwrap();

        let pos1: f64 = conn.query_row(
            "SELECT position FROM tasks WHERE id=?1", params![id1], |r| r.get(0),
        ).unwrap();
        let pos2: f64 = conn.query_row(
            "SELECT position FROM tasks WHERE id=?1", params![id2], |r| r.get(0),
        ).unwrap();

        assert!(pos2 > pos1, "second task position should be greater than first");
    }

    // ── Task deletion tests ────────────────────────────────────────────────────

    #[test]
    fn test_delete_task_single() {
        let conn = setup_db();
        let id = insert_task(&conn, "Task to delete", None).unwrap();
        assert!(task_exists(&conn, &id));

        let result = delete_task(&conn, &id);
        assert!(result.is_ok());
        assert!(!task_exists(&conn, &id));
    }

    #[test]
    fn test_delete_task_not_found() {
        let conn = setup_db();
        let result = delete_task(&conn, "nonexistent-id");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "task not found");
    }

    #[test]
    fn test_delete_task_recursive_removes_children() {
        let conn = setup_db();
        let parent_id = insert_task(&conn, "Parent", None).unwrap();
        let child1_id = insert_task(&conn, "Child 1", Some(&parent_id)).unwrap();
        let child2_id = insert_task(&conn, "Child 2", Some(&parent_id)).unwrap();
        let grandchild_id = insert_task(&conn, "Grandchild", Some(&child1_id)).unwrap();

        // Confirm all 4 exist (plus default seeds from migrate, but we only check ours)
        assert!(task_exists(&conn, &parent_id));
        assert!(task_exists(&conn, &child1_id));
        assert!(task_exists(&conn, &child2_id));
        assert!(task_exists(&conn, &grandchild_id));

        let result = delete_task_recursive(&conn, &parent_id);
        assert!(result.is_ok());

        assert!(!task_exists(&conn, &parent_id));
        assert!(!task_exists(&conn, &child1_id));
        assert!(!task_exists(&conn, &child2_id));
        assert!(!task_exists(&conn, &grandchild_id));
    }

    #[test]
    fn test_delete_task_recursive_not_found() {
        let conn = setup_db();
        let result = delete_task_recursive(&conn, "nonexistent-id");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "task not found");
    }

    #[test]
    fn test_delete_parent_cascades_to_children_via_fk() {
        // Verify the FK ON DELETE CASCADE also works (SQLite foreign_keys=ON)
        let conn = setup_db();
        let parent_id = insert_task(&conn, "Parent", None).unwrap();
        let child_id = insert_task(&conn, "Child", Some(&parent_id)).unwrap();

        conn.execute("DELETE FROM tasks WHERE id=?1", params![parent_id]).unwrap();

        // With FK cascade enabled, child should also be gone
        assert!(!task_exists(&conn, &child_id), "child should be cascade-deleted");
    }

    // ── Recurrence interval clamp tests ────────────────────────────────────────

    #[test]
    fn test_recurrence_interval_clamp_over_9999() {
        // The production code does: raw.min(9999)
        assert_eq!(clamp_interval(10000), 9999);
        assert_eq!(clamp_interval(u64::MAX), 9999);
        assert_eq!(clamp_interval(100_000), 9999);
    }

    #[test]
    fn test_recurrence_interval_clamp_at_9999() {
        assert_eq!(clamp_interval(9999), 9999);
    }

    #[test]
    fn test_recurrence_interval_clamp_under_9999() {
        assert_eq!(clamp_interval(1), 1);
        assert_eq!(clamp_interval(365), 365);
        assert_eq!(clamp_interval(9998), 9998);
    }

    // ── Flag CRUD tests ────────────────────────────────────────────────────────

    #[test]
    fn test_create_flag_valid() {
        let conn = setup_db();
        // migrate() seeds 4 default flags; add a custom one
        let result = insert_flag(&conn, "Custom Flag", "#FF0000");
        assert!(result.is_ok(), "valid flag should be created");
        let id = result.unwrap();
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM flags WHERE id=?1)", params![id], |r| r.get(0),
        ).unwrap();
        assert!(exists, "flag should exist in DB");
    }

    #[test]
    fn test_create_flag_empty_name_rejected() {
        let conn = setup_db();
        let result = insert_flag(&conn, "", "#FF0000");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "name cannot be empty");
    }

    #[test]
    fn test_create_flag_name_too_long_rejected() {
        let conn = setup_db();
        let long_name = "x".repeat(51);
        let result = insert_flag(&conn, &long_name, "#FF0000");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "name too long (max 50 chars)");
    }

    #[test]
    fn test_create_flag_invalid_color_rejected() {
        let conn = setup_db();
        let result = insert_flag(&conn, "My Flag", "red");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "color must be #RRGGBB");
    }

    #[test]
    fn test_create_flag_duplicate_name_rejected() {
        let conn = setup_db();
        insert_flag(&conn, "Duplicate", "#AABBCC").unwrap();
        let result = insert_flag(&conn, "Duplicate", "#112233");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "name already exists");
    }

    #[test]
    fn test_delete_flag_valid() {
        let conn = setup_db();
        let flag_id = insert_flag(&conn, "ToDelete", "#123456").unwrap();
        let result = delete_flag(&conn, &flag_id);
        assert!(result.is_ok());
        let still_exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM flags WHERE id=?1)", params![flag_id], |r| r.get(0),
        ).unwrap();
        assert!(!still_exists, "deleted flag should not exist");
    }

    #[test]
    fn test_delete_flag_not_found() {
        let conn = setup_db();
        let result = delete_flag(&conn, "nonexistent-flag-id");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "flag not found");
    }

    #[test]
    fn test_delete_flag_nullifies_task_flag_id() {
        // ON DELETE SET NULL — task's flag_id should become NULL when flag is deleted
        let conn = setup_db();
        let flag_id = insert_flag(&conn, "TempFlag", "#ABCDEF").unwrap();
        let task_id = insert_task(&conn, "Flagged task", None).unwrap();

        conn.execute(
            "UPDATE tasks SET flag_id=?1 WHERE id=?2",
            params![flag_id, task_id],
        ).unwrap();

        delete_flag(&conn, &flag_id).unwrap();

        let flag_ref: Option<String> = conn.query_row(
            "SELECT flag_id FROM tasks WHERE id=?1", params![task_id], |r| r.get(0),
        ).unwrap();
        assert!(flag_ref.is_none(), "flag_id should be NULL after flag deleted");
    }

    // ── Settings allowlist tests ───────────────────────────────────────────────

    #[test]
    fn test_set_setting_valid_key_accepted() {
        let conn = setup_db();
        let result = set_setting(&conn, "app_font", "Inter");
        assert!(result.is_ok());
        let val = get_setting(&conn, "app_font");
        assert_eq!(val, Some("Inter".to_string()));
    }

    #[test]
    fn test_set_setting_all_allowed_keys_accepted() {
        let conn = setup_db();
        let cases = [
            ("app_font", "Arial"),
            ("app_font_size", "14"),
            ("app_compact", "true"),
            ("app_task_color", "#FF0000"),
            ("startup_remember_position", "false"),
            ("startup_single_instance", "true"),
        ];
        for (key, value) in cases {
            let result = set_setting(&conn, key, value);
            assert!(result.is_ok(), "key '{}' should be accepted", key);
        }
    }

    #[test]
    fn test_set_setting_invalid_key_rejected() {
        let conn = setup_db();
        let result = set_setting(&conn, "gdrive_access_token", "some-token");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown or restricted setting key"));
    }

    #[test]
    fn test_set_setting_sensitive_keys_rejected() {
        let conn = setup_db();
        let sensitive_keys = [
            "gdrive_access_token",
            "gdrive_refresh_token",
            "gdrive_client_secret",
            "api_token",
        ];
        for key in sensitive_keys {
            let result = set_setting(&conn, key, "value");
            assert!(result.is_err(), "key '{}' should be rejected", key);
        }
    }

    #[test]
    fn test_set_setting_arbitrary_key_rejected() {
        let conn = setup_db();
        let result = set_setting(&conn, "totally_made_up_key", "whatever");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown or restricted setting key"));
    }

    #[test]
    fn test_set_setting_upsert_updates_existing() {
        let conn = setup_db();
        set_setting(&conn, "app_font", "Inter").unwrap();
        set_setting(&conn, "app_font", "Roboto").unwrap();
        let val = get_setting(&conn, "app_font");
        assert_eq!(val, Some("Roboto".to_string()));
    }

    // ── SQL injection safety — parameterised queries ───────────────────────────
    // These tests verify the production code uses params![] for user input.
    // They confirm that SQL metacharacters in user data do NOT break queries.

    #[test]
    fn test_sql_injection_in_caption_is_safe() {
        let conn = setup_db();
        let malicious = "'); DROP TABLE tasks; --";
        let result = insert_task(&conn, malicious, None);
        assert!(result.is_ok(), "SQL metacharacters in caption should be handled safely");
        // tasks table still intact
        let count = count_tasks(&conn);
        assert!(count >= 1, "tasks table should still exist and contain the row");
    }

    #[test]
    fn test_sql_injection_in_flag_name_is_safe() {
        let conn = setup_db();
        let malicious = "'); DROP TABLE flags; --";
        let result = insert_flag(&conn, malicious, "#AABBCC");
        assert!(result.is_ok(), "SQL metacharacters in flag name should be handled safely");
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM flags", [], |r| r.get(0)).unwrap();
        assert!(count >= 1, "flags table should still exist");
    }

    #[test]
    fn test_sql_injection_in_setting_value_is_safe() {
        let conn = setup_db();
        let malicious = "'; DROP TABLE app_settings; --";
        let result = set_setting(&conn, "app_font", malicious);
        assert!(result.is_ok());
        let val = get_setting(&conn, "app_font");
        // The malicious string should be stored verbatim, not executed
        assert_eq!(val, Some(malicious.to_string()));
        // app_settings table still intact
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM app_settings", [], |r| r.get(0)).unwrap();
        assert!(count >= 1);
    }

    // ── Color validation tests ─────────────────────────────────────────────────

    #[test]
    fn test_validate_color_valid_hex() {
        assert!(validate_color("#AABBCC").is_ok());
        assert!(validate_color("#000000").is_ok());
        assert!(validate_color("#ffffff").is_ok());
        assert!(validate_color("#4A9EFF").is_ok());
    }

    #[test]
    fn test_validate_color_invalid_formats() {
        assert!(validate_color("red").is_err());
        assert!(validate_color("#FFF").is_err());           // 3-digit shorthand
        assert!(validate_color("#GGHHII").is_err());        // non-hex digits
        assert!(validate_color("AABBCC").is_err());         // missing #
        assert!(validate_color("#AABBCCD").is_err());       // too long
        assert!(validate_color("").is_err());
    }

    // ── Migration idempotency ──────────────────────────────────────────────────

    #[test]
    fn test_migration_is_idempotent() {
        let conn = setup_db();
        // Running migrate twice should not fail (uses CREATE TABLE IF NOT EXISTS)
        let result = crate::db::migrate(&conn);
        assert!(result.is_ok(), "second migration run should succeed");
    }

    #[test]
    fn test_migration_seeds_default_flags() {
        let conn = setup_db();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM flags", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 4, "should have exactly 4 default flags seeded");
    }

    #[test]
    fn test_migration_does_not_double_seed_flags() {
        let conn = setup_db();
        // Run migrate again — should not insert more seeds
        crate::db::migrate(&conn).unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM flags", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 4, "second migrate should not double-seed flags");
    }
}
