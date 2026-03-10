use rusqlite::{params, Connection};
use std::sync::Mutex;
use tauri::State;
use uuid::Uuid;
use chrono::Utc;

use crate::types::*;

pub struct DbState(pub Mutex<Connection>);

fn load_task_extras(conn: &Connection, task_id: &str) -> (Vec<Context>, Vec<Tag>, Vec<EmailLink>) {
    let contexts = conn.prepare(
        "SELECT c.id, c.name, c.color, c.position FROM contexts c
         JOIN task_contexts tc ON c.id = tc.context_id
         WHERE tc.task_id = ?1 ORDER BY c.position"
    ).and_then(|mut s| {
        s.query_map(params![task_id], |r| Ok(Context {
            id: r.get(0)?,
            name: r.get(1)?,
            color: r.get(2)?,
            position: r.get(3)?,
        })).map(|rows| rows.filter_map(|r| r.ok()).collect())
    }).unwrap_or_default();

    let tags = conn.prepare(
        "SELECT t.id, t.name, t.color FROM tags t
         JOIN task_tags tt ON t.id = tt.tag_id
         WHERE tt.task_id = ?1"
    ).and_then(|mut s| {
        s.query_map(params![task_id], |r| Ok(Tag {
            id: r.get(0)?,
            name: r.get(1)?,
            color: r.get(2)?,
        })).map(|rows| rows.filter_map(|r| r.ok()).collect())
    }).unwrap_or_default();

    let email_links = conn.prepare(
        "SELECT id, task_id, link_type, link_data, subject FROM email_links WHERE task_id = ?1"
    ).and_then(|mut s| {
        s.query_map(params![task_id], |r| Ok(EmailLink {
            id: r.get(0)?,
            task_id: r.get(1)?,
            link_type: r.get(2)?,
            link_data: r.get(3)?,
            subject: r.get(4)?,
        })).map(|rows| rows.filter_map(|r| r.ok()).collect())
    }).unwrap_or_default();

    (contexts, tags, email_links)
}

fn row_to_task(conn: &Connection, id: String, parent_id: Option<String>, caption: String,
    note: String, position: f64, created_at: String, updated_at: String,
    completed_at: Option<String>, importance: i32, urgency: i32, effort: i32,
    due_date: Option<String>, reminder_at: Option<String>, recurrence_rule: Option<String>,
    starred: bool, color: Option<String>) -> Task
{
    let has_children: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM tasks WHERE parent_id = ?1 AND completed_at IS NULL)",
        params![&id], |r| r.get(0)
    ).unwrap_or(false);

    let score = compute_score(importance, urgency, &due_date);
    let (contexts, tags, email_links) = load_task_extras(conn, &id);

    Task {
        id, parent_id, caption, note, position, created_at, updated_at, completed_at,
        importance, urgency, effort, due_date, reminder_at, recurrence_rule,
        starred, color, contexts, tags, email_links, has_children, score,
    }
}

#[tauri::command]
pub fn get_tasks(state: State<DbState>, parent_id: Option<String>) -> Vec<Task> {
    let conn = state.0.lock().unwrap();
    let sql = "SELECT id, parent_id, caption, note, position, created_at, updated_at,
                completed_at, importance, urgency, effort, due_date, reminder_at,
                recurrence_rule, starred, color
               FROM tasks WHERE parent_id IS ?1 AND completed_at IS NULL
               ORDER BY position";
    conn.prepare(sql).and_then(|mut s| {
        s.query_map(params![parent_id], |r| {
            Ok((
                r.get::<_,String>(0)?, r.get::<_,Option<String>>(1)?,
                r.get::<_,String>(2)?, r.get::<_,String>(3)?,
                r.get::<_,f64>(4)?, r.get::<_,String>(5)?, r.get::<_,String>(6)?,
                r.get::<_,Option<String>>(7)?, r.get::<_,i32>(8)?,
                r.get::<_,i32>(9)?, r.get::<_,i32>(10)?,
                r.get::<_,Option<String>>(11)?, r.get::<_,Option<String>>(12)?,
                r.get::<_,Option<String>>(13)?, r.get::<_,bool>(14)?,
                r.get::<_,Option<String>>(15)?
            ))
        }).map(|rows| {
            rows.filter_map(|r| r.ok())
                .map(|(id, pid, cap, note, pos, ca, ua, compl, imp, urg, eff, due, rem, rec, star, col)| {
                    row_to_task(&conn, id, pid, cap, note, pos, ca, ua, compl, imp, urg, eff, due, rem, rec, star, col)
                }).collect()
        })
    }).unwrap_or_default()
}

#[tauri::command]
pub fn get_all_tasks_flat(state: State<DbState>) -> Vec<Task> {
    let conn = state.0.lock().unwrap();
    let sql = "SELECT id, parent_id, caption, note, position, created_at, updated_at,
                completed_at, importance, urgency, effort, due_date, reminder_at,
                recurrence_rule, starred, color
               FROM tasks WHERE completed_at IS NULL ORDER BY position";
    conn.prepare(sql).and_then(|mut s| {
        s.query_map([], |r| {
            Ok((
                r.get::<_,String>(0)?, r.get::<_,Option<String>>(1)?,
                r.get::<_,String>(2)?, r.get::<_,String>(3)?,
                r.get::<_,f64>(4)?, r.get::<_,String>(5)?, r.get::<_,String>(6)?,
                r.get::<_,Option<String>>(7)?, r.get::<_,i32>(8)?,
                r.get::<_,i32>(9)?, r.get::<_,i32>(10)?,
                r.get::<_,Option<String>>(11)?, r.get::<_,Option<String>>(12)?,
                r.get::<_,Option<String>>(13)?, r.get::<_,bool>(14)?,
                r.get::<_,Option<String>>(15)?
            ))
        }).map(|rows| {
            rows.filter_map(|r| r.ok())
                .map(|(id, pid, cap, note, pos, ca, ua, compl, imp, urg, eff, due, rem, rec, star, col)| {
                    row_to_task(&conn, id, pid, cap, note, pos, ca, ua, compl, imp, urg, eff, due, rem, rec, star, col)
                }).collect()
        })
    }).unwrap_or_default()
}

#[tauri::command]
pub fn create_task(state: State<DbState>, input: CreateTaskInput) -> Result<Task, String> {
    let conn = state.0.lock().unwrap();
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let position = input.position.unwrap_or_else(|| {
        let max: f64 = conn.query_row(
            "SELECT COALESCE(MAX(position), 0) FROM tasks WHERE parent_id IS ?1",
            params![input.parent_id],
            |r| r.get(0)
        ).unwrap_or(0.0);
        max + 1000.0
    });

    conn.execute(
        "INSERT INTO tasks (id, parent_id, caption, note, position, created_at, updated_at,
          importance, urgency, effort, due_date, starred, color)
         VALUES (?1,?2,?3,?4,?5,?6,?6,?7,?8,?9,?10,?11,?12)",
        params![
            id, input.parent_id, input.caption,
            input.note.unwrap_or_default(),
            position, now,
            input.importance.unwrap_or(3),
            input.urgency.unwrap_or(3),
            input.effort.unwrap_or(3),
            input.due_date,
            input.starred.unwrap_or(false),
            input.color,
        ],
    ).map_err(|e| e.to_string())?;

    if let Some(ctx_ids) = &input.context_ids {
        for cid in ctx_ids {
            conn.execute(
                "INSERT OR IGNORE INTO task_contexts (task_id, context_id) VALUES (?1, ?2)",
                params![id, cid],
            ).ok();
        }
    }

    get_task_by_id(&conn, &id).ok_or("Task not found after insert".into())
}

#[tauri::command]
pub fn update_task(state: State<DbState>, id: String, input: UpdateTaskInput) -> Result<Task, String> {
    let conn = state.0.lock().unwrap();
    let now = Utc::now().to_rfc3339();

    if let Some(cap) = &input.caption {
        conn.execute("UPDATE tasks SET caption=?1, updated_at=?2 WHERE id=?3",
            params![cap, now, id]).ok();
    }
    if let Some(note) = &input.note {
        conn.execute("UPDATE tasks SET note=?1, updated_at=?2 WHERE id=?3",
            params![note, now, id]).ok();
    }
    if let Some(v) = input.importance {
        conn.execute("UPDATE tasks SET importance=?1, updated_at=?2 WHERE id=?3",
            params![v, now, id]).ok();
    }
    if let Some(v) = input.urgency {
        conn.execute("UPDATE tasks SET urgency=?1, updated_at=?2 WHERE id=?3",
            params![v, now, id]).ok();
    }
    if let Some(v) = input.effort {
        conn.execute("UPDATE tasks SET effort=?1, updated_at=?2 WHERE id=?3",
            params![v, now, id]).ok();
    }
    if let Some(v) = &input.due_date {
        let val: Option<&str> = if v.is_empty() { None } else { Some(v) };
        conn.execute("UPDATE tasks SET due_date=?1, updated_at=?2 WHERE id=?3",
            params![val, now, id]).ok();
    }
    if let Some(v) = &input.reminder_at {
        conn.execute("UPDATE tasks SET reminder_at=?1, updated_at=?2 WHERE id=?3",
            params![v, now, id]).ok();
    }
    if let Some(v) = &input.recurrence_rule {
        conn.execute("UPDATE tasks SET recurrence_rule=?1, updated_at=?2 WHERE id=?3",
            params![v, now, id]).ok();
    }
    if let Some(v) = input.starred {
        conn.execute("UPDATE tasks SET starred=?1, updated_at=?2 WHERE id=?3",
            params![v, now, id]).ok();
    }
    if let Some(v) = &input.color {
        conn.execute("UPDATE tasks SET color=?1, updated_at=?2 WHERE id=?3",
            params![v, now, id]).ok();
    }
    if let Some(ctx_ids) = &input.context_ids {
        conn.execute("DELETE FROM task_contexts WHERE task_id=?1", params![id]).ok();
        for cid in ctx_ids {
            conn.execute("INSERT OR IGNORE INTO task_contexts (task_id, context_id) VALUES (?1,?2)",
                params![id, cid]).ok();
        }
    }
    if let Some(tag_ids) = &input.tag_ids {
        conn.execute("DELETE FROM task_tags WHERE task_id=?1", params![id]).ok();
        for tid in tag_ids {
            conn.execute("INSERT OR IGNORE INTO task_tags (task_id, tag_id) VALUES (?1,?2)",
                params![id, tid]).ok();
        }
    }

    get_task_by_id(&conn, &id).ok_or("Task not found".into())
}

#[tauri::command]
pub fn delete_task(state: State<DbState>, id: String) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    conn.execute("DELETE FROM tasks WHERE id=?1", params![id])
        .map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn complete_task(state: State<DbState>, id: String, completed: bool) -> Result<Task, String> {
    let conn = state.0.lock().unwrap();
    let now = Utc::now().to_rfc3339();
    let completed_at: Option<String> = if completed { Some(now.clone()) } else { None };
    conn.execute("UPDATE tasks SET completed_at=?1, updated_at=?2 WHERE id=?3",
        params![completed_at, now, id]).map_err(|e| e.to_string())?;
    get_task_by_id(&conn, &id).ok_or("Task not found".into())
}

#[tauri::command]
pub fn move_task(state: State<DbState>, id: String, new_parent_id: Option<String>, new_position: f64) -> Result<Task, String> {
    let conn = state.0.lock().unwrap();
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE tasks SET parent_id=?1, position=?2, updated_at=?3 WHERE id=?4",
        params![new_parent_id, new_position, now, id]
    ).map_err(|e| e.to_string())?;
    get_task_by_id(&conn, &id).ok_or("Task not found".into())
}

#[tauri::command]
pub fn reorder_tasks(state: State<DbState>, ids_and_positions: Vec<(String, f64)>) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    let now = Utc::now().to_rfc3339();
    for (id, pos) in ids_and_positions {
        conn.execute("UPDATE tasks SET position=?1, updated_at=?2 WHERE id=?3",
            params![pos, now, id]).ok();
    }
    Ok(())
}

fn get_task_by_id(conn: &Connection, id: &str) -> Option<Task> {
    conn.query_row(
        "SELECT id, parent_id, caption, note, position, created_at, updated_at,
          completed_at, importance, urgency, effort, due_date, reminder_at,
          recurrence_rule, starred, color FROM tasks WHERE id=?1",
        params![id],
        |r| Ok((
            r.get::<_,String>(0)?, r.get::<_,Option<String>>(1)?,
            r.get::<_,String>(2)?, r.get::<_,String>(3)?,
            r.get::<_,f64>(4)?, r.get::<_,String>(5)?, r.get::<_,String>(6)?,
            r.get::<_,Option<String>>(7)?, r.get::<_,i32>(8)?,
            r.get::<_,i32>(9)?, r.get::<_,i32>(10)?,
            r.get::<_,Option<String>>(11)?, r.get::<_,Option<String>>(12)?,
            r.get::<_,Option<String>>(13)?, r.get::<_,bool>(14)?,
            r.get::<_,Option<String>>(15)?
        ))
    ).ok().map(|(id, pid, cap, note, pos, ca, ua, compl, imp, urg, eff, due, rem, rec, star, col)| {
        row_to_task(conn, id, pid, cap, note, pos, ca, ua, compl, imp, urg, eff, due, rem, rec, star, col)
    })
}
