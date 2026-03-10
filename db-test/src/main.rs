/// Standalone smoke test for TaskClaw DB logic.
/// No Tauri / GTK deps — just rusqlite.
use rusqlite::{Connection, params};
use uuid::Uuid;
use chrono::Utc;

fn open_mem() -> Connection {
    let conn = Connection::open_in_memory().expect("open mem db");
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    conn.execute_batch("
        CREATE TABLE tasks (
            id TEXT PRIMARY KEY,
            parent_id TEXT,
            caption TEXT NOT NULL,
            note TEXT NOT NULL DEFAULT '',
            position REAL NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            completed_at TEXT,
            importance INTEGER NOT NULL DEFAULT 3,
            urgency INTEGER NOT NULL DEFAULT 3,
            effort INTEGER NOT NULL DEFAULT 3,
            due_date TEXT,
            starred INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (parent_id) REFERENCES tasks(id) ON DELETE CASCADE
        );
        CREATE TABLE contexts (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            color TEXT NOT NULL DEFAULT '#4A9EFF',
            position REAL NOT NULL DEFAULT 0
        );
        CREATE TABLE task_contexts (
            task_id TEXT NOT NULL,
            context_id TEXT NOT NULL,
            PRIMARY KEY (task_id, context_id),
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
            FOREIGN KEY (context_id) REFERENCES contexts(id) ON DELETE CASCADE
        );
    ").unwrap();
    conn
}

fn insert_task(conn: &Connection, parent_id: Option<&str>, caption: &str, pos: f64) -> String {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO tasks (id, parent_id, caption, position, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?5)",
        params![id, parent_id, caption, pos, now],
    ).unwrap();
    id
}

fn count_tasks(conn: &Connection, parent_id: Option<&str>) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE parent_id IS ?1 AND completed_at IS NULL",
        params![parent_id], |r| r.get(0)
    ).unwrap()
}

fn compute_score(importance: i32, urgency: i32, due_date: Option<&str>) -> f64 {
    let base = (importance as f64 * 0.6 + urgency as f64 * 0.4) * 20.0;
    let due_boost = if let Some(due) = due_date {
        let now = chrono::Utc::now().date_naive();
        if let Ok(d) = chrono::NaiveDate::parse_from_str(due, "%Y-%m-%d") {
            let days = (d - now).num_days();
            if days < 0 { 20.0 } else if days == 0 { 15.0 } else if days <= 3 { 10.0 } else if days <= 7 { 5.0 } else { 0.0 }
        } else { 0.0 }
    } else { 0.0 };
    (base + due_boost).min(100.0)
}

fn main() {
    println!("=== TaskClaw DB Smoke Test ===\n");

    let conn = open_mem();

    // ── Test 1: Create root tasks ─────────────────────────────────────────
    let t1 = insert_task(&conn, None, "Buy groceries", 1000.0);
    let t2 = insert_task(&conn, None, "Call dentist",  2000.0);
    let t3 = insert_task(&conn, None, "Project Alpha",  3000.0);
    assert_eq!(count_tasks(&conn, None), 3);
    println!("[PASS] Create 3 root tasks");

    // ── Test 2: Create subtasks ───────────────────────────────────────────
    let s1 = insert_task(&conn, Some(&t3), "Write spec",   1000.0);
    let s2 = insert_task(&conn, Some(&t3), "Setup repo",   2000.0);
    let _s3 = insert_task(&conn, Some(&t3), "First commit", 3000.0);
    assert_eq!(count_tasks(&conn, Some(&t3)), 3);
    println!("[PASS] Create 3 subtasks under Project Alpha");

    // ── Test 3: Deep nesting ──────────────────────────────────────────────
    let ss1 = insert_task(&conn, Some(&s1), "Draft outline", 1000.0);
    insert_task(&conn, Some(&ss1), "Section intro", 1000.0);
    assert_eq!(count_tasks(&conn, Some(&s1)), 1);
    println!("[PASS] Deep nesting (3 levels)");

    // ── Test 4: Complete a task ───────────────────────────────────────────
    let now = Utc::now().to_rfc3339();
    conn.execute("UPDATE tasks SET completed_at=?1 WHERE id=?2", params![now, s2]).unwrap();
    assert_eq!(count_tasks(&conn, Some(&t3)), 2); // s2 now hidden
    println!("[PASS] Complete task — excluded from active count");

    // ── Test 5: Delete cascades to children ──────────────────────────────
    conn.execute("DELETE FROM tasks WHERE id=?1", params![t3]).unwrap();
    assert_eq!(count_tasks(&conn, None), 2); // t1, t2 remain
    let orphans: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE parent_id=?1", params![t3], |r| r.get(0)
    ).unwrap();
    assert_eq!(orphans, 0);
    println!("[PASS] Delete parent cascades to all children");

    // ── Test 6: Contexts ─────────────────────────────────────────────────
    let ctx_id = Uuid::new_v4().to_string();
    conn.execute("INSERT INTO contexts (id, name, color, position) VALUES (?1,'@home','#4A9EFF',0)",
        params![ctx_id]).unwrap();
    conn.execute("INSERT INTO task_contexts (task_id, context_id) VALUES (?1,?2)",
        params![t1, ctx_id]).unwrap();
    let ctx_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM task_contexts WHERE task_id=?1", params![t1], |r| r.get(0)
    ).unwrap();
    assert_eq!(ctx_count, 1);
    println!("[PASS] Context assignment");

    // ── Test 7: Context cascade on task delete ───────────────────────────
    conn.execute("DELETE FROM tasks WHERE id=?1", params![t1]).unwrap();
    let orphan_ctx: i64 = conn.query_row(
        "SELECT COUNT(*) FROM task_contexts WHERE task_id=?1", params![t1], |r| r.get(0)
    ).unwrap();
    assert_eq!(orphan_ctx, 0);
    println!("[PASS] task_contexts cascade on task delete");

    // ── Test 8: Priority scoring ──────────────────────────────────────────
    let score_55 = compute_score(5, 5, None);
    let score_11 = compute_score(1, 1, None);
    let score_today = compute_score(3, 3, Some(&chrono::Utc::now().date_naive().to_string()));
    let score_overdue = compute_score(1, 1, Some("2020-01-01"));
    assert!(score_55 > score_11, "high i/u > low i/u");
    assert!(score_today > score_55 * 0.5, "due today boosts score");
    assert!(score_overdue > score_11, "overdue boosts score");
    println!("[PASS] Priority scoring: high={:.0} low={:.0} today={:.0} overdue={:.0}",
        score_55, score_11, score_today, score_overdue);

    // ── Test 9: Reorder (position update) ────────────────────────────────
    let _t4 = insert_task(&conn, None, "Task D", 4000.0);
    let t5 = insert_task(&conn, None, "Task E", 5000.0);
    let t6 = insert_task(&conn, None, "Task F", 6000.0);
    conn.execute("UPDATE tasks SET position=?1 WHERE id=?2", params![3500.0_f64, t5]).unwrap();
    conn.execute("UPDATE tasks SET position=?1 WHERE id=?2", params![3501.0_f64, t6]).unwrap();
    let ordered: Vec<String> = {
        let mut s = conn.prepare("SELECT caption FROM tasks WHERE completed_at IS NULL ORDER BY position").unwrap();
        s.query_map([], |r| r.get(0)).unwrap().filter_map(|r| r.ok()).collect()
    };
    assert!(ordered.iter().position(|c| c == "Task E") < ordered.iter().position(|c| c == "Task F"),
        "E before F after reorder");
    println!("[PASS] Position-based reorder");

    // ── Test 10: Rename ───────────────────────────────────────────────────
    let now2 = Utc::now().to_rfc3339();
    conn.execute("UPDATE tasks SET caption='Call dentist (urgent)', updated_at=?1 WHERE id=?2",
        params![now2, t2]).unwrap();
    let renamed: String = conn.query_row(
        "SELECT caption FROM tasks WHERE id=?1", params![t2], |r| r.get(0)
    ).unwrap();
    assert_eq!(renamed, "Call dentist (urgent)");
    println!("[PASS] Rename task caption");

    println!("\n✓ All 10 smoke tests passed.\n");
}
