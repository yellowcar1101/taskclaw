use rusqlite::{params, Connection};
use std::sync::Mutex;
use tauri::State;
use uuid::Uuid;
use chrono::{Utc, NaiveDate, Datelike, Duration, Months, Local};

use crate::types::*;

pub struct DbState(pub Mutex<Connection>);

// ── helpers ───────────────────────────────────────────────────────────────────

fn load_flag(conn: &Connection, flag_id: &Option<String>) -> Option<Flag> {
    let id = flag_id.as_ref()?;
    conn.query_row(
        "SELECT id, name, color, position FROM flags WHERE id = ?1",
        params![id],
        |r| Ok(Flag { id: r.get(0)?, name: r.get(1)?, color: r.get(2)?, position: r.get(3)? }),
    ).ok()
}

fn load_tags(conn: &Connection, task_id: &str) -> Vec<Tag> {
    conn.prepare(
        "SELECT t.id, t.name, t.color FROM tags t
         JOIN task_tags tt ON t.id = tt.tag_id
         WHERE tt.task_id = ?1"
    ).and_then(|mut s| {
        s.query_map(params![task_id], |r| Ok(Tag {
            id: r.get(0)?, name: r.get(1)?, color: r.get(2)?,
        })).map(|rows| rows.filter_map(|r| r.ok()).collect())
    }).unwrap_or_default()
}

fn load_email_links(conn: &Connection, task_id: &str) -> Vec<EmailLink> {
    conn.prepare(
        "SELECT id, task_id, link_type, link_data, subject FROM email_links WHERE task_id = ?1"
    ).and_then(|mut s| {
        s.query_map(params![task_id], |r| Ok(EmailLink {
            id: r.get(0)?, task_id: r.get(1)?, link_type: r.get(2)?,
            link_data: r.get(3)?, subject: r.get(4)?,
        })).map(|rows| rows.filter_map(|r| r.ok()).collect())
    }).unwrap_or_default()
}

fn has_children_query(conn: &Connection, id: &str) -> bool {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM tasks WHERE parent_id = ?1 AND completed_at IS NULL)",
        params![id], |r| r.get(0)
    ).unwrap_or(false)
}

#[allow(clippy::too_many_arguments)]
fn row_to_task(
    conn: &Connection,
    id: String, parent_id: Option<String>, caption: String, note: String,
    position: f64, created_at: String, updated_at: String, completed_at: Option<String>,
    start_date: Option<String>, due_date: Option<String>, reminder_at: Option<String>,
    recurrence_rule: Option<String>, flag_id: Option<String>,
    starred: bool, color: Option<String>,
    is_folder: bool, is_project: bool, hide_in_views: bool,
    subtasks_in_order: bool, inherit_dates: bool, custom_format: Option<String>,
) -> Task {
    let flag = load_flag(conn, &flag_id);
    let tags = load_tags(conn, &id);
    let email_links = load_email_links(conn, &id);
    let has_children = has_children_query(conn, &id);
    Task {
        id, parent_id, caption, note, position, created_at, updated_at, completed_at,
        start_date, due_date, reminder_at, recurrence_rule, flag_id, flag,
        starred, color, is_folder, is_project, hide_in_views,
        subtasks_in_order, inherit_dates, custom_format,
        tags, email_links, has_children,
    }
}

const TASK_SELECT: &str = "
    SELECT id, parent_id, caption, note, position, created_at, updated_at, completed_at,
           start_date, due_date, reminder_at, recurrence_rule, flag_id,
           starred, color, is_folder, is_project, hide_in_views,
           subtasks_in_order, inherit_dates, custom_format
    FROM tasks";

fn map_row(r: &rusqlite::Row) -> rusqlite::Result<(
    String, Option<String>, String, String, f64, String, String, Option<String>,
    Option<String>, Option<String>, Option<String>, Option<String>, Option<String>,
    bool, Option<String>, bool, bool, bool, bool, bool, Option<String>,
)> {
    Ok((
        r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?,
        r.get(5)?, r.get(6)?, r.get(7)?,
        r.get(8)?, r.get(9)?, r.get(10)?, r.get(11)?, r.get(12)?,
        r.get::<_,i32>(13)? != 0,
        r.get(14)?,
        r.get::<_,i32>(15)? != 0,
        r.get::<_,i32>(16)? != 0,
        r.get::<_,i32>(17)? != 0,
        r.get::<_,i32>(18)? != 0,
        r.get::<_,i32>(19)? != 0,
        r.get(20)?,
    ))
}

fn hydrate(conn: &Connection, rows: Vec<(
    String, Option<String>, String, String, f64, String, String, Option<String>,
    Option<String>, Option<String>, Option<String>, Option<String>, Option<String>,
    bool, Option<String>, bool, bool, bool, bool, bool, Option<String>,
)>) -> Vec<Task> {
    rows.into_iter().map(|(id, pid, cap, note, pos, ca, ua, compl,
        sd, due, rem, rec, fid, star, col, ifolder, iproj, hiv, sio, inh, cf)| {
        row_to_task(conn, id, pid, cap, note, pos, ca, ua, compl,
            sd, due, rem, rec, fid, star, col, ifolder, iproj, hiv, sio, inh, cf)
    }).collect()
}

fn get_task_by_id(conn: &Connection, id: &str) -> Option<Task> {
    let sql = format!("{} WHERE id = ?1", TASK_SELECT);
    conn.query_row(&sql, params![id], map_row).ok().map(|row| {
        let (id, pid, cap, note, pos, ca, ua, compl, sd, due, rem, rec, fid, star, col,
             ifolder, iproj, hiv, sio, inh, cf) = row;
        row_to_task(conn, id, pid, cap, note, pos, ca, ua, compl,
            sd, due, rem, rec, fid, star, col, ifolder, iproj, hiv, sio, inh, cf)
    })
}

// ── commands ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_tasks(state: State<DbState>, parent_id: Option<String>) -> Vec<Task> {
    let conn = state.0.lock().unwrap();
    let sql = format!("{} WHERE parent_id IS ?1 AND completed_at IS NULL ORDER BY position", TASK_SELECT);
    conn.prepare(&sql).and_then(|mut s| {
        s.query_map(params![parent_id], map_row)
            .map(|rows| hydrate(&conn, rows.filter_map(|r| r.ok()).collect()))
    }).unwrap_or_default()
}

#[tauri::command]
pub fn get_all_tasks_flat(state: State<DbState>, include_completed: Option<bool>) -> Vec<Task> {
    let conn = state.0.lock().unwrap();
    let where_clause = if include_completed.unwrap_or(false) { "" } else { " WHERE completed_at IS NULL" };
    let sql = format!("{}{} ORDER BY position", TASK_SELECT, where_clause);
    conn.prepare(&sql).and_then(|mut s| {
        s.query_map([], map_row)
            .map(|rows| hydrate(&conn, rows.filter_map(|r| r.ok()).collect()))
    }).unwrap_or_default()
}

#[tauri::command]
pub fn create_task(state: State<DbState>, input: CreateTaskInput) -> Result<Task, String> {
    let conn = state.0.lock().unwrap();

    if input.caption.is_empty() { return Err("caption cannot be empty".into()); }
    if input.caption.len() > 500 { return Err("caption too long (max 500 chars)".into()); }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let position = input.position.unwrap_or_else(|| {
        let max: f64 = conn.query_row(
            "SELECT COALESCE(MAX(position), 0) FROM tasks WHERE parent_id IS ?1",
            params![input.parent_id], |r| r.get(0),
        ).unwrap_or(0.0);
        max + 1.0
    });

    let is_folder = input.is_folder.unwrap_or(false);
    let is_project = if is_folder { false } else { input.is_project.unwrap_or(false) };

    conn.execute(
        "INSERT INTO tasks (id, parent_id, caption, note, position, created_at, updated_at,
          start_date, due_date, reminder_at, flag_id, starred, is_folder, is_project)
         VALUES (?1,?2,?3,?4,?5,?6,?6,?7,?8,?9,?10,?11,?12,?13)",
        params![
            id, input.parent_id, input.caption,
            input.note.unwrap_or_default(),
            position, now,
            input.start_date, input.due_date, input.reminder_at,
            input.flag_id,
            input.starred.unwrap_or(false) as i32,
            is_folder as i32, is_project as i32,
        ],
    ).map_err(|e| e.to_string())?;

    if let Some(tag_ids) = &input.tag_ids {
        for tid in tag_ids {
            conn.execute(
                "INSERT OR IGNORE INTO task_tags (task_id, tag_id) VALUES (?1, ?2)",
                params![id, tid],
            ).ok();
        }
    }

    get_task_by_id(&conn, &id).ok_or_else(|| "task not found after insert".into())
}

#[tauri::command]
pub fn update_task(state: State<DbState>, id: String, input: UpdateTaskInput) -> Result<Task, String> {
    let conn = state.0.lock().unwrap();
    let now = Utc::now().to_rfc3339();

    // verify exists
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM tasks WHERE id=?1)", params![id], |r| r.get(0)
    ).unwrap_or(false);
    if !exists { return Err("task not found".into()); }

    macro_rules! set_field {
        ($field:expr, $val:expr) => {
            conn.execute(
                &format!("UPDATE tasks SET {}=?1, updated_at=?2 WHERE id=?3", $field),
                params![$val, now, id],
            ).map_err(|e| e.to_string())?;
        };
    }

    if let Some(v) = &input.caption {
        if v.is_empty() { return Err("caption cannot be empty".into()); }
        if v.len() > 500 { return Err("caption too long (max 500 chars)".into()); }
        set_field!("caption", v);
    }
    if let Some(v) = &input.note { set_field!("note", v); }
    if let Some(v) = &input.start_date {
        set_field!("start_date", if v.is_empty() { None } else { Some(v.as_str()) });
    }
    if let Some(v) = &input.due_date {
        set_field!("due_date", if v.is_empty() { None } else { Some(v.as_str()) });
    }
    if let Some(v) = &input.reminder_at {
        set_field!("reminder_at", if v.is_empty() { None } else { Some(v.as_str()) });
    }
    if let Some(v) = &input.recurrence_rule {
        if !v.is_empty() {
            serde_json::from_str::<serde_json::Value>(v)
                .map_err(|_| "recurrence_rule is not valid JSON")?;
        }
        set_field!("recurrence_rule", if v.is_empty() { None } else { Some(v.as_str()) });
    }
    if let Some(v) = &input.flag_id {
        set_field!("flag_id", if v.is_empty() { None } else { Some(v.as_str()) });
    }
    if let Some(v) = input.starred { set_field!("starred", v as i32); }
    if let Some(v) = &input.color {
        set_field!("color", if v.is_empty() { None } else { Some(v.as_str()) });
    }
    if let Some(v) = input.is_folder {
        set_field!("is_folder", v as i32);
        if v { set_field!("is_project", 0i32); }
    }
    if let Some(v) = input.is_project {
        set_field!("is_project", v as i32);
        if v { set_field!("is_folder", 0i32); }
    }
    if let Some(v) = input.hide_in_views   { set_field!("hide_in_views",      v as i32); }
    if let Some(v) = input.subtasks_in_order { set_field!("subtasks_in_order", v as i32); }
    if let Some(v) = input.inherit_dates   { set_field!("inherit_dates",       v as i32); }
    if let Some(v) = &input.custom_format {
        if !v.is_empty() {
            serde_json::from_str::<serde_json::Value>(v)
                .map_err(|_| "custom_format is not valid JSON")?;
        }
        set_field!("custom_format", if v.is_empty() { None } else { Some(v.as_str()) });
    }
    if let Some(tag_ids) = &input.tag_ids {
        conn.execute("DELETE FROM task_tags WHERE task_id=?1", params![id]).ok();
        for tid in tag_ids {
            conn.execute("INSERT OR IGNORE INTO task_tags (task_id, tag_id) VALUES (?1,?2)",
                params![id, tid]).ok();
        }
    }

    // always refresh updated_at
    conn.execute("UPDATE tasks SET updated_at=?1 WHERE id=?2", params![now, id]).ok();

    get_task_by_id(&conn, &id).ok_or_else(|| "task not found".into())
}

#[tauri::command]
pub fn delete_task(state: State<DbState>, id: String) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    let n = conn.execute("DELETE FROM tasks WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    if n == 0 { Err("task not found".into()) } else { Ok(()) }
}

#[tauri::command]
pub fn delete_task_recursive(state: State<DbState>, id: String) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM tasks WHERE id=?1)", params![id], |r| r.get(0)
    ).unwrap_or(false);
    if !exists { return Err("task not found".into()); }
    conn.execute_batch(&format!("
        WITH RECURSIVE descendants(id) AS (
            SELECT id FROM tasks WHERE id = '{}'
            UNION ALL
            SELECT t.id FROM tasks t JOIN descendants d ON t.parent_id = d.id
        )
        DELETE FROM tasks WHERE id IN (SELECT id FROM descendants);
    ", id.replace('\'', "''"))).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn complete_task(state: State<DbState>, id: String, completed: bool) -> Result<Task, String> {
    let conn = state.0.lock().unwrap();
    let now = Utc::now().to_rfc3339();

    // If marking complete and task has recurrence, advance instead of archiving
    if completed {
        let task = get_task_by_id(&conn, &id).ok_or("task not found")?;
        if let Some(rule_str) = &task.recurrence_rule {
            if let Ok(rule) = serde_json::from_str::<serde_json::Value>(rule_str) {
                let no_completed_copy = rule["no_completed_copy"].as_bool().unwrap_or(false);
                let reset_subtasks   = rule["reset_subtasks"].as_bool().unwrap_or(false);
                let regenerate       = rule["regenerate"].as_bool().unwrap_or(false);

                // Create a completed snapshot unless disabled
                if !no_completed_copy {
                    let copy_id = Uuid::new_v4().to_string();
                    conn.execute(
                        "INSERT INTO tasks (id, parent_id, caption, note, position, created_at, updated_at,
                          completed_at, start_date, due_date, reminder_at, recurrence_rule, flag_id,
                          starred, color, is_folder, is_project, hide_in_views, subtasks_in_order,
                          inherit_dates, custom_format)
                         SELECT ?1, parent_id, caption, note, position - 0.1, created_at, ?2,
                          ?2, start_date, due_date, reminder_at, NULL, flag_id,
                          starred, color, is_folder, is_project, hide_in_views, subtasks_in_order,
                          inherit_dates, custom_format
                         FROM tasks WHERE id = ?3",
                        params![copy_id, now, id],
                    ).map_err(|e| e.to_string())?;
                    // Copy tags
                    let tag_ids: Vec<String> = conn.prepare(
                        "SELECT tag_id FROM task_tags WHERE task_id = ?1"
                    ).and_then(|mut s| {
                        s.query_map(params![id], |r| r.get(0))
                         .map(|rows| rows.filter_map(|r| r.ok()).collect())
                    }).unwrap_or_default();
                    for tid in &tag_ids {
                        conn.execute(
                            "INSERT OR IGNORE INTO task_tags (task_id, tag_id) VALUES (?1, ?2)",
                            params![copy_id, tid],
                        ).ok();
                    }
                }

                // Determine base date for next occurrence
                let today = Local::now().date_naive();
                let base = if regenerate {
                    today
                } else {
                    task.due_date.as_deref()
                        .and_then(|d| NaiveDate::parse_from_str(&d[..10], "%Y-%m-%d").ok())
                        .unwrap_or(today)
                };

                let next_due = next_occurrence_from(base, &rule);
                let next_start = if let (Some(start_str), Some(due_str), Some(ref nd)) =
                    (&task.start_date, &task.due_date, &next_due)
                {
                    let s = NaiveDate::parse_from_str(&start_str[..10], "%Y-%m-%d").ok();
                    let d = NaiveDate::parse_from_str(&due_str[..10], "%Y-%m-%d").ok();
                    if let (Some(sv), Some(dv)) = (s, d) {
                        let offset = (dv - sv).num_days();
                        Some((*nd - Duration::days(offset)).format("%Y-%m-%d").to_string())
                    } else { None }
                } else { None };

                let next_due_str = next_due.map(|d| d.format("%Y-%m-%d").to_string());
                conn.execute(
                    "UPDATE tasks SET due_date=?1, start_date=?2, completed_at=NULL, updated_at=?3 WHERE id=?4",
                    params![next_due_str, next_start, now, id],
                ).map_err(|e| e.to_string())?;

                if reset_subtasks {
                    conn.execute_batch(&format!(
                        "WITH RECURSIVE sub(id) AS (
                            SELECT id FROM tasks WHERE parent_id = '{0}'
                            UNION ALL
                            SELECT t.id FROM tasks t JOIN sub s ON t.parent_id = s.id
                         )
                         UPDATE tasks SET completed_at = NULL, updated_at = '{1}' WHERE id IN (SELECT id FROM sub)",
                        id.replace('\'', "''"), now.replace('\'', "''")
                    )).map_err(|e| e.to_string())?;
                }

                return get_task_by_id(&conn, &id).ok_or_else(|| "task not found after recurrence advance".into());
            }
        }
    }

    // Normal complete / uncomplete
    let completed_at: Option<&str> = if completed { Some(&now) } else { None };
    let n = conn.execute(
        "UPDATE tasks SET completed_at=?1, updated_at=?2 WHERE id=?3",
        params![completed_at, now, id],
    ).map_err(|e| e.to_string())?;
    if n == 0 { return Err("task not found".into()); }
    get_task_by_id(&conn, &id).ok_or_else(|| "task not found".into())
}

#[tauri::command]
pub fn complete_branch(state: State<DbState>, id: String, completed: bool) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM tasks WHERE id=?1)", params![id], |r| r.get(0)
    ).unwrap_or(false);
    if !exists { return Err("task not found".into()); }
    let now = Utc::now().to_rfc3339();
    let completed_at_sql = if completed {
        format!("'{}'", now)
    } else {
        "NULL".to_string()
    };
    conn.execute_batch(&format!("
        WITH RECURSIVE branch(id) AS (
            SELECT id FROM tasks WHERE id = '{}'
            UNION ALL
            SELECT t.id FROM tasks t JOIN branch b ON t.parent_id = b.id
        )
        UPDATE tasks SET completed_at = {}, updated_at = '{}'
        WHERE id IN (SELECT id FROM branch);
    ", id.replace('\'', "''"), completed_at_sql, now.replace('\'', "''")
    )).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn move_task(state: State<DbState>, id: String, new_parent_id: Option<String>, new_position: f64) -> Result<Task, String> {
    let conn = state.0.lock().unwrap();
    let now = Utc::now().to_rfc3339();
    // cycle check
    if let Some(ref npid) = new_parent_id {
        if npid == &id { return Err("cannot move task into its own descendant".into()); }
        let is_desc: bool = conn.query_row(
            "WITH RECURSIVE desc(id) AS (
               SELECT id FROM tasks WHERE id = ?1
               UNION ALL
               SELECT t.id FROM tasks t JOIN desc d ON t.parent_id = d.id
             )
             SELECT EXISTS(SELECT 1 FROM desc WHERE id = ?2)",
            params![id, npid], |r| r.get(0)
        ).unwrap_or(false);
        if is_desc { return Err("cannot move task into its own descendant".into()); }
    }
    let n = conn.execute(
        "UPDATE tasks SET parent_id=?1, position=?2, updated_at=?3 WHERE id=?4",
        params![new_parent_id, new_position, now, id],
    ).map_err(|e| e.to_string())?;
    if n == 0 { return Err("task not found".into()); }
    get_task_by_id(&conn, &id).ok_or_else(|| "task not found".into())
}

#[tauri::command]
pub fn reorder_tasks(state: State<DbState>, ids_and_positions: Vec<(String, f64)>) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    let now = Utc::now().to_rfc3339();
    for (id, pos) in &ids_and_positions {
        let n = conn.execute(
            "UPDATE tasks SET position=?1, updated_at=?2 WHERE id=?3",
            params![pos, now, id],
        ).map_err(|e| e.to_string())?;
        if n == 0 { return Err(format!("task {} not found", id)); }
    }
    Ok(())
}

#[tauri::command]
pub fn duplicate_task(state: State<DbState>, id: String) -> Result<Task, String> {
    let conn = state.0.lock().unwrap();
    let original = get_task_by_id(&conn, &id).ok_or("task not found")?;
    let new_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO tasks (id, parent_id, caption, note, position, created_at, updated_at,
          start_date, due_date, reminder_at, recurrence_rule, flag_id, starred, color,
          is_folder, is_project, hide_in_views, subtasks_in_order, inherit_dates, custom_format)
         SELECT ?1, parent_id, caption, note, position + 0.5, ?2, ?2,
          start_date, due_date, reminder_at, recurrence_rule, flag_id, starred, color,
          is_folder, is_project, hide_in_views, subtasks_in_order, inherit_dates, custom_format
         FROM tasks WHERE id = ?3",
        params![new_id, now, id],
    ).map_err(|e| e.to_string())?;
    // copy tags
    for tag in &original.tags {
        conn.execute(
            "INSERT OR IGNORE INTO task_tags (task_id, tag_id) VALUES (?1, ?2)",
            params![new_id, tag.id],
        ).ok();
    }
    get_task_by_id(&conn, &new_id).ok_or_else(|| "task not found after duplicate".into())
}

#[tauri::command]
pub fn sort_subtasks(state: State<DbState>, parent_id: Option<String>, sort_by: String, sort_dir: String) -> Result<(), String> {
    let valid_sort = ["caption", "due_date", "start_date", "created_at", "flag", "starred"];
    if !valid_sort.contains(&sort_by.as_str()) { return Err("invalid sort_by value".into()); }
    if sort_dir != "asc" && sort_dir != "desc" { return Err("invalid sort_dir value".into()); }

    let conn = state.0.lock().unwrap();
    let sql = format!("{} WHERE parent_id IS ?1", TASK_SELECT);
    let mut tasks: Vec<Task> = conn.prepare(&sql).and_then(|mut s| {
        s.query_map(params![parent_id], map_row)
            .map(|rows| hydrate(&conn, rows.filter_map(|r| r.ok()).collect()))
    }).unwrap_or_default();

    tasks.sort_by(|a, b| {
        let cmp = match sort_by.as_str() {
            "caption"    => a.caption.to_lowercase().cmp(&b.caption.to_lowercase()),
            "due_date"   => a.due_date.as_deref().unwrap_or("9999").cmp(b.due_date.as_deref().unwrap_or("9999")),
            "start_date" => a.start_date.as_deref().unwrap_or("9999").cmp(b.start_date.as_deref().unwrap_or("9999")),
            "created_at" => a.created_at.cmp(&b.created_at),
            "flag"       => a.flag.as_ref().map(|f| f.position as i64).unwrap_or(i64::MAX)
                             .cmp(&b.flag.as_ref().map(|f| f.position as i64).unwrap_or(i64::MAX)),
            "starred"    => b.starred.cmp(&a.starred), // starred first regardless of dir
            _            => std::cmp::Ordering::Equal,
        };
        if sort_dir == "desc" && sort_by != "starred" { cmp.reverse() } else { cmp }
    });

    let now = Utc::now().to_rfc3339();
    for (i, task) in tasks.iter().enumerate() {
        conn.execute(
            "UPDATE tasks SET position=?1, updated_at=?2 WHERE id=?3",
            params![(i + 1) as f64, now, task.id],
        ).ok();
    }
    Ok(())
}

// ── Recurrence helpers ────────────────────────────────────────────────────────
// recurrence_rule JSON format:
//   freq: "daily"|"weekly"|"monthly"|"yearly"
//   interval: N (default 1)
//   days_of_week: [0..6]  (0=Mon 6=Sun, ISO; weekly only)
//   day_of_month: 1..31   (monthly: specific day)
//   nth_weekday: {n:1..5, day:0..6}  (monthly: e.g. 2nd Monday)
//   month: 1..12          (yearly: which month)
//   regenerate: bool      (base next occurrence on completion date, not due date)
//   reset_subtasks: bool
//   no_completed_copy: bool

fn days_in_month(year: i32, month: u32) -> u32 {
    let (ny, nm) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    NaiveDate::from_ymd_opt(ny, nm, 1)
        .map(|d| (d - Duration::days(1)).day())
        .unwrap_or(30)
}

fn nth_weekday_of_month(year: i32, month: u32, n: u32, day_iso: u32) -> Option<NaiveDate> {
    // day_iso: 0=Mon..6=Sun (ISO weekday num_days_from_monday)
    let first = NaiveDate::from_ymd_opt(year, month, 1)?;
    let first_iso = first.weekday().num_days_from_monday(); // 0=Mon
    let days_ahead = (day_iso + 7 - first_iso) % 7;
    let first_occ = 1 + days_ahead;
    let target = first_occ + (n - 1) * 7;
    if target > days_in_month(year, month) { return None; }
    NaiveDate::from_ymd_opt(year, month, target)
}

fn next_occurrence_from(base: NaiveDate, rule: &serde_json::Value) -> Option<NaiveDate> {
    let freq = rule["freq"].as_str()?;
    let interval = rule["interval"].as_u64().unwrap_or(1);

    match freq {
        "daily" => Some(base + Duration::days(interval as i64)),

        "weekly" => {
            if let Some(arr) = rule["days_of_week"].as_array() {
                if !arr.is_empty() {
                    let day_nums: Vec<u32> = arr.iter()
                        .filter_map(|d| d.as_u64().map(|v| v as u32))
                        .collect();
                    // Search up to interval weeks ahead
                    let limit = base + Duration::days(interval as i64 * 7 + 7);
                    let mut candidate = base + Duration::days(1);
                    while candidate <= limit {
                        if day_nums.contains(&candidate.weekday().num_days_from_monday()) {
                            return Some(candidate);
                        }
                        candidate += Duration::days(1);
                    }
                }
            }
            Some(base + Duration::days(interval as i64 * 7))
        }

        "monthly" => {
            let months = Months::new(interval as u32);
            if let Some(dom) = rule["day_of_month"].as_u64() {
                let advanced = base.checked_add_months(months)?;
                let max_day = days_in_month(advanced.year(), advanced.month());
                NaiveDate::from_ymd_opt(advanced.year(), advanced.month(), (dom as u32).min(max_day))
            } else if let Some(nth) = rule["nth_weekday"].as_object() {
                let n   = nth.get("n").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
                let day = nth.get("day").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let advanced = base.checked_add_months(months)?;
                nth_weekday_of_month(advanced.year(), advanced.month(), n, day)
            } else {
                base.checked_add_months(months)
            }
        }

        "yearly" => {
            let new_year = base.year() + interval as i32;
            let (target_month, target_day) = if let Some(m) = rule["month"].as_u64() {
                let mo = m as u32;
                (mo, base.day().min(days_in_month(new_year, mo)))
            } else {
                (base.month(), base.day().min(days_in_month(new_year, base.month())))
            };
            NaiveDate::from_ymd_opt(new_year, target_month, target_day)
        }

        _ => None,
    }
}

#[tauri::command]
pub fn skip_occurrence(state: State<DbState>, id: String) -> Result<Task, String> {
    let conn = state.0.lock().unwrap();
    let task = get_task_by_id(&conn, &id).ok_or("task not found")?;

    let rule_str = task.recurrence_rule.as_deref().ok_or("task has no recurrence rule")?;
    let rule: serde_json::Value = serde_json::from_str(rule_str)
        .map_err(|_| "invalid recurrence_rule JSON")?;

    let today = Local::now().date_naive();
    let base = task.due_date.as_deref()
        .and_then(|d| NaiveDate::parse_from_str(&d[..10], "%Y-%m-%d").ok())
        .unwrap_or(today);

    let next_due = next_occurrence_from(base, &rule);
    let next_start = if let (Some(start_str), Some(due_str), Some(nd)) =
        (&task.start_date, &task.due_date, next_due)
    {
        let s = NaiveDate::parse_from_str(&start_str[..10], "%Y-%m-%d").ok();
        let d = NaiveDate::parse_from_str(&due_str[..10], "%Y-%m-%d").ok();
        if let (Some(sv), Some(dv)) = (s, d) {
            let offset = (dv - sv).num_days();
            Some((nd - Duration::days(offset)).format("%Y-%m-%d").to_string())
        } else { None }
    } else { None };

    let next_due_str = next_due.map(|d| d.format("%Y-%m-%d").to_string());
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE tasks SET due_date=?1, start_date=?2, updated_at=?3 WHERE id=?4",
        params![next_due_str, next_start, now, id],
    ).map_err(|e| e.to_string())?;

    get_task_by_id(&conn, &id).ok_or_else(|| "task not found after update".into())
}
