# TaskClaw — Full Product Specification

> Version: 2.0 | App version: 0.3.0 | Status: Phase 2 complete — active development
> Audience: Senior developer. This document is the single source of truth. Build exactly what is described; do not add features not listed; do not omit features that are listed.

---

## 1. Overview

TaskClaw is a portable Windows desktop task manager built with **Tauri v2** (Rust backend) + **SvelteKit** (frontend). It is a spiritual clone of MyLifeOrganized (MLO), optimised for keyboard-first, power-user workflows.

**Key principles:**
- Portable: the entire app (exe + data) lives in one folder. No registry entries, no AppData traces.
- Offline-first: all data is local SQLite. GDrive sync is optional.
- Keyboard-driven: every common action has a shortcut or can be reached without a mouse.
- Dense UI: small fonts, compact rows, no wasted space.

---

## 2. Tech Stack

| Layer | Technology |
|---|---|
| Desktop shell | Tauri v2 |
| Frontend | SvelteKit (SSR disabled), TypeScript |
| Styling | Plain CSS with CSS custom properties (no Tailwind, no CSS-in-JS) |
| State management | Svelte stores (`writable`, `derived`) |
| Database | SQLite via `rusqlite` with `bundled` feature |
| Build / CI | GitHub Actions, `windows-latest`, Rust stable, Node 20 |
| Sync backend | Google Drive REST API (OAuth2, loopback flow) |

**Tauri commands** are the only bridge between frontend and Rust. All data access goes through `invoke()`. No direct filesystem access from the frontend.

---

## 3. File & Folder Structure

```
TaskClaw.exe
Data/
  tasks.db          ← SQLite database (auto-created on first run)
  tasks.db-wal      ← WAL journal (normal SQLite artifact)
  tasks.db-shm      ← WAL shared memory (normal SQLite artifact)
```

DB path in Rust: `std::env::current_exe() → parent() → join("Data/tasks.db")`.
`Data/` directory created with `std::fs::create_dir_all` if missing.

---

## 4. Design System

### 4.1 CSS Custom Properties

Defined in `src/app.css` on `:root`. All components consume these variables — never hardcode colours.

```css
:root {
  /* Backgrounds */
  --bg:               #1a1a1a;   /* main content background */
  --surface:          #222222;   /* titlebar, sidebars */
  --surface-elevated: #2a2a2a;   /* toolbar, column headers */
  --input-bg:         #272727;   /* inputs, selects, textareas */

  /* Borders */
  --border:           #3c3c3c;   /* all borders */

  /* Text */
  --text:             #e0e0e0;   /* primary text */
  --text-dim:         #888888;   /* labels, placeholders, secondary */

  /* Interactive */
  --accent:           #4A9EFF;   /* primary accent (blue) */
  --accent-dim:       #4A9EFF33; /* accent at 20% opacity */
  --hover:            #ffffff0f; /* row/button hover */
  --hover-btn:        #2e2e2e;   /* button default background */
  --selected:         #4A9EFF1a; /* selected row background */

  /* Semantic colours */
  --red:              #E05C5C;   /* danger, overdue */
  --amber:            #D4A843;   /* warning, today */
  --green:            #6ABF69;   /* success */
}
```

### 4.2 Typography

- **UI font**: system-ui / sans-serif, 12px base
- **Monospace** (app name, code): `'Cascadia Code', 'Fira Code', monospace`
- Task captions: 13px
- Labels / metadata: 11–12px
- Line height: 1.4

### 4.3 Spacing & Sizing

- Row height (task row): 28px
- Titlebar height: 36px
- Toolbar height: 32px
- Column header height: 22px
- Border radius (buttons, inputs): 4px
- Border radius (modals): 8px
- Standard padding (panels): 8px

### 4.4 Iconography

Use Unicode characters — no icon library dependency.

| Symbol | Usage |
|---|---|
| `▶` / `▼` | Expand / collapse |
| `☰` | Outline tab |
| `⚙` | Preferences |
| `⭐` | Star |
| `📋` | Rapid Input |
| `⊞` / `⊟` | Expand all / Collapse all |
| `✕` | Close / Clear |
| `✎` | Edit |
| `+` | Add |
| `🗑` | Delete |

---

## 5. Database Schema

### 5.1 Full Schema (exact SQL)

```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS flags (
  id       TEXT PRIMARY KEY,
  name     TEXT NOT NULL,
  color    TEXT NOT NULL DEFAULT '#4A9EFF',  -- hex colour
  position REAL NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS tasks (
  id               TEXT PRIMARY KEY,
  parent_id        TEXT REFERENCES tasks(id) ON DELETE CASCADE,
  caption          TEXT NOT NULL,
  note             TEXT NOT NULL DEFAULT '',
  position         REAL NOT NULL DEFAULT 0,
  created_at       TEXT NOT NULL,   -- ISO 8601 UTC
  updated_at       TEXT NOT NULL,   -- ISO 8601 UTC
  completed_at     TEXT,            -- NULL = incomplete
  start_date       TEXT,            -- YYYY-MM-DD or YYYY-MM-DDTHH:mm
  due_date         TEXT,            -- YYYY-MM-DD or YYYY-MM-DDTHH:mm
  reminder_at      TEXT,            -- YYYY-MM-DDTHH:mm (always has time)
  recurrence_rule  TEXT,            -- JSON blob (see §5.2)
  flag_id          TEXT REFERENCES flags(id) ON DELETE SET NULL,
  starred          INTEGER NOT NULL DEFAULT 0,  -- 0 or 1
  color            TEXT,            -- hex, task-level custom colour
  is_folder        INTEGER NOT NULL DEFAULT 0,
  is_project       INTEGER NOT NULL DEFAULT 0,
  hide_in_views    INTEGER NOT NULL DEFAULT 0,
  subtasks_in_order INTEGER NOT NULL DEFAULT 0,
  inherit_dates    INTEGER NOT NULL DEFAULT 0,
  custom_format    TEXT             -- JSON blob (see §5.3)
);

CREATE TABLE IF NOT EXISTS tags (
  id    TEXT PRIMARY KEY,
  name  TEXT NOT NULL UNIQUE,
  color TEXT NOT NULL DEFAULT '#888888'
);

CREATE TABLE IF NOT EXISTS task_tags (
  task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  tag_id  TEXT NOT NULL REFERENCES tags(id)  ON DELETE CASCADE,
  PRIMARY KEY (task_id, tag_id)
);

CREATE TABLE IF NOT EXISTS email_links (
  id        TEXT PRIMARY KEY,
  task_id   TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  link_type TEXT NOT NULL,   -- 'outlook' | 'url'
  link_data TEXT NOT NULL,
  subject   TEXT
);

CREATE TABLE IF NOT EXISTS saved_views (
  id              TEXT PRIMARY KEY,
  name            TEXT NOT NULL,
  show_completed  INTEGER NOT NULL DEFAULT 0,
  group_by        TEXT NOT NULL DEFAULT 'none',  -- 'none'|'flag'|'tag'|'due'|'start'
  sort_by         TEXT NOT NULL DEFAULT 'position', -- 'position'|'due'|'start'|'caption'|'created_at'
  sort_dir        TEXT NOT NULL DEFAULT 'asc',   -- 'asc'|'desc'
  visible_fields  TEXT NOT NULL DEFAULT '[]',    -- JSON array of field names
  filter_json     TEXT NOT NULL DEFAULT '{}',    -- JSON filter object (see §5.4)
  position        REAL NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS app_settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL   -- JSON-encoded value
);

CREATE INDEX IF NOT EXISTS idx_tasks_parent    ON tasks(parent_id);
CREATE INDEX IF NOT EXISTS idx_tasks_due       ON tasks(due_date);
CREATE INDEX IF NOT EXISTS idx_tasks_start     ON tasks(start_date);
CREATE INDEX IF NOT EXISTS idx_tasks_flag      ON tasks(flag_id);
CREATE INDEX IF NOT EXISTS idx_tasks_reminder  ON tasks(reminder_at);
```

### 5.2 Recurrence Rule JSON

Stored in `tasks.recurrence_rule` as a JSON string.

```jsonc
{
  "pattern": "weekly",          // "hourly"|"daily"|"weekly"|"monthly"|"yearly"
  "interval": 1,                // recur every N units
  "days_of_week": [3],          // for weekly: 0=Mon … 6=Sun
  "day_of_month": null,         // for monthly
  "regenerate": false,          // true = regenerate N units after completion
  "start_date": "2026-04-03T00:00",
  "due_date":   "2026-04-03T00:00",
  "lead_time_days": 0,
  "use_time": false,
  "lock_period": false,
  "end_mode": "none",           // "none"|"after_n"|"by_date"
  "end_after_n": null,          // integer if end_mode = "after_n"
  "end_by_date": null,          // ISO date if end_mode = "by_date"
  "occurrences_completed": 0,   // counter
  "advanced": {
    "subtask_reset": "reset_all",      // "none"|"reset_all"|"reset_if_all_complete"
    "auto_recur": "disabled",          // "disabled"|"on_any_subtask"|"on_all_subtasks"
    "no_completed_copy": true
  }
}
```

### 5.3 Custom Format JSON

Stored in `tasks.custom_format`.

```jsonc
{
  "bold": false,
  "italic": false,
  "underline": false,
  "strikethrough": false,
  "highlight_color": null,   // hex or null
  "font_color": null,        // hex or null
  "sidebar_color": null,     // hex or null
  "subtasks_inherit": false
}
```

### 5.4 View Filter JSON

Stored in `saved_views.filter_json`.

```jsonc
{
  "flag_ids": [],        // [] = all flags; ["id1","id2"] = only these
  "tag_ids": [],
  "starred_only": false,
  "has_due_date": null,  // null|true|false
  "due_before": null,    // ISO date or null
  "due_after": null
}
```

### 5.5 App Settings Keys

| Key | Value type | Default | Description |
|---|---|---|---|
| `sync_auto_enabled` | bool | false | Auto-sync on/off |
| `sync_interval_min` | int | 15 | Auto-sync interval in minutes |
| `sync_last_at` | string | null | ISO timestamp of last successful sync |
| `detail_panel_order` | string[] | ["notes","general","timing","format"] | Section order in Task Detail |
| `detail_panel_hidden` | string[] | [] | Hidden section IDs |
| `last_flag_used` | string | null | ID of last flag applied via single-click |
| `reminder_dismissed` | string[] | [] | Task IDs permanently dismissed |
| `reminder_snoozed` | object | {} | `{taskId: ISO_datetime}` |

### 5.6 Seed Data

Insert on first run (when `flags` table is empty):

```sql
INSERT INTO flags (id, name, color, position) VALUES
  (uuid(), '🔴 Urgent',    '#E05C5C', 0),
  (uuid(), '🟡 Review',    '#D4A843', 1),
  (uuid(), '🔵 Waiting',   '#4A9EFF', 2),
  (uuid(), '🟢 Delegated', '#6ABF69', 3);
```

---

## 6. Rust Backend

### 6.1 Module Structure

```
src-tauri/src/
  main.rs           ← entry point (6 lines, calls lib::run())
  lib.rs            ← registers all commands, opens DB
  db.rs             ← db_path(), open(), migrate()
  types.rs          ← all serialisable structs
  commands/
    mod.rs
    tasks.rs        ← task CRUD + DbState
    flags.rs        ← flags, tags, views, email links
    sync.rs         ← GDrive sync commands
    settings.rs     ← app settings get/set
```

### 6.2 DbState

```rust
pub struct DbState(pub Mutex<Connection>);
```

One connection, WAL mode, foreign keys on. All commands lock the mutex for the duration of the call.

### 6.3 Full Command List

#### Tasks

| Command | Params | Returns |
|---|---|---|
| `get_tasks` | `parent_id: Option<String>` | `Vec<Task>` |
| `get_all_tasks_flat` | `include_completed: Option<bool>` | `Vec<Task>` |
| `create_task` | `input: CreateTaskInput` | `Result<Task>` |
| `update_task` | `id: String, input: UpdateTaskInput` | `Result<Task>` |
| `delete_task` | `id: String` | `Result<()>` |
| `delete_task_recursive` | `id: String` | `Result<()>` — deletes task + all descendants |
| `complete_task` | `id: String, completed: bool` | `Result<Task>` |
| `complete_branch` | `id: String, completed: bool` | `Result<()>` — complete/uncomplete task + all subtasks |
| `move_task` | `id: String, new_parent_id: Option<String>, new_position: f64` | `Result<Task>` |
| `reorder_tasks` | `ids_and_positions: Vec<(String, f64)>` | `Result<()>` |
| `duplicate_task` | `id: String` | `Result<Task>` — shallow copy (no subtasks) |
| `sort_subtasks` | `parent_id: Option<String>, sort_by: String, sort_dir: String` | `Result<()>` |
| `skip_occurrence` | `id: String` | `Result<Task>` — advance recurring task to next occurrence |

#### Flags

| Command | Params | Returns |
|---|---|---|
| `get_flags` | — | `Vec<Flag>` |
| `create_flag` | `name: String, color: String` | `Result<Flag>` |
| `update_flag` | `id: String, name: String, color: String` | `Result<Flag>` |
| `delete_flag` | `id: String` | `Result<()>` |
| `reorder_flags` | `ids_and_positions: Vec<(String, f64)>` | `Result<()>` |

#### Tags

| Command | Params | Returns |
|---|---|---|
| `get_tags` | — | `Vec<Tag>` |
| `create_tag` | `name: String, color: String` | `Result<Tag>` |
| `update_tag` | `id: String, name: String, color: String` | `Result<Tag>` |
| `delete_tag` | `id: String` | `Result<()>` |

#### Views

| Command | Params | Returns |
|---|---|---|
| `get_views` | — | `Vec<SavedView>` |
| `create_view` | `payload: ViewPayload` | `Result<SavedView>` |
| `update_view` | `id: String, payload: ViewPayload` | `Result<SavedView>` |
| `delete_view` | `id: String` | `Result<()>` |
| `reorder_views` | `ids_and_positions: Vec<(String, f64)>` | `Result<()>` |

#### Email Links

| Command | Params | Returns |
|---|---|---|
| `add_email_link` | `task_id, link_type, link_data, subject: Option<String>` | `Result<String>` (new id) |
| `delete_email_link` | `id: String` | `Result<()>` |

#### Settings

| Command | Params | Returns |
|---|---|---|
| `get_setting` | `key: String` | `Option<String>` |
| `set_setting` | `key: String, value: String` | `Result<()>` |
| `get_all_settings` | — | `HashMap<String, String>` |

#### Sync

| Command | Params | Returns |
|---|---|---|
| `gdrive_auth_status` | — | `bool` |
| `gdrive_connect` | — | `Result<SyncResult>` |
| `gdrive_upload` | — | `Result<SyncResult>` |
| `gdrive_download` | — | `Result<SyncResult>` |
| `gdrive_sync` | — | `Result<SyncResult>` — smart sync with conflict detection |
| `gdrive_disconnect` | — | `Result<()>` |

### 6.4 Type Definitions

```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: String,
    pub parent_id: Option<String>,
    pub caption: String,
    pub note: String,
    pub position: f64,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub start_date: Option<String>,
    pub due_date: Option<String>,
    pub reminder_at: Option<String>,
    pub recurrence_rule: Option<String>,  // raw JSON string
    pub flag_id: Option<String>,
    pub flag: Option<Flag>,               // eagerly loaded
    pub starred: bool,
    pub color: Option<String>,
    pub is_folder: bool,
    pub is_project: bool,
    pub hide_in_views: bool,
    pub subtasks_in_order: bool,
    pub inherit_dates: bool,
    pub custom_format: Option<String>,    // raw JSON string
    pub tags: Vec<Tag>,                   // eagerly loaded
    pub email_links: Vec<EmailLink>,      // eagerly loaded
    pub has_children: bool,               // computed
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Flag { pub id: String, pub name: String, pub color: String, pub position: f64 }

#[derive(Serialize, Deserialize, Clone)]
pub struct Tag { pub id: String, pub name: String, pub color: String }

#[derive(Serialize, Deserialize, Clone)]
pub struct EmailLink {
    pub id: String, pub task_id: String,
    pub link_type: String, pub link_data: String, pub subject: Option<String>
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SavedView {
    pub id: String, pub name: String,
    pub show_completed: bool, pub group_by: String,
    pub sort_by: String, pub sort_dir: String,
    pub visible_fields: Vec<String>,
    pub filter_json: String,
    pub position: f64,
}

#[derive(Deserialize)]
pub struct CreateTaskInput {
    pub parent_id: Option<String>,
    pub caption: String,
    pub note: Option<String>,
    pub position: Option<f64>,
    pub start_date: Option<String>,
    pub due_date: Option<String>,
    pub reminder_at: Option<String>,
    pub flag_id: Option<String>,
    pub starred: Option<bool>,
    pub tag_ids: Option<Vec<String>>,
    pub is_folder: Option<bool>,
    pub is_project: Option<bool>,
}

#[derive(Deserialize)]
pub struct UpdateTaskInput {
    pub caption: Option<String>,
    pub note: Option<String>,
    pub start_date: Option<String>,   // empty string = clear
    pub due_date: Option<String>,     // empty string = clear
    pub reminder_at: Option<String>,  // empty string = clear
    pub recurrence_rule: Option<String>,
    pub flag_id: Option<String>,      // empty string = clear
    pub starred: Option<bool>,
    pub color: Option<String>,
    pub is_folder: Option<bool>,
    pub is_project: Option<bool>,
    pub hide_in_views: Option<bool>,
    pub subtasks_in_order: Option<bool>,
    pub inherit_dates: Option<bool>,
    pub custom_format: Option<String>,
    pub tag_ids: Option<Vec<String>>,
}
```

### 6.5 API Contract Detail

This section documents each Tauri command with full request/response shapes, field-level validation, error conditions, and edge-case behaviour. All commands return `Result<T, String>` on the Rust side, serialised by Tauri as `{ data: T }` on success or a rejected Promise with a string error message on failure.

---

#### `get_tasks`

```
Params:  { parent_id?: string | null }
Returns: Task[]
Errors:  none (returns [] on DB error)
```

- `parent_id` omitted or `null` → return all root tasks (`parent_id IS NULL`), ordered by `position ASC`.
- `parent_id` provided → return direct children of that task only (one level deep).
- Each returned `Task` has `flag`, `tags`, and `email_links` eagerly loaded.
- `has_children` is computed as `EXISTS(SELECT 1 FROM tasks WHERE parent_id = t.id AND completed_at IS NULL)`.
- Completed tasks are excluded from the default result. To include them, use `get_all_tasks_flat`.

---

#### `get_all_tasks_flat`

```
Params:  { include_completed?: boolean }   default: false
Returns: Task[]
Errors:  none
```

- Returns every task in the database as a flat array, ordered by `position ASC` within each parent group.
- `include_completed: false` (default) → `WHERE completed_at IS NULL`.
- `include_completed: true` → all rows.
- Each row has full eager loads (flag, tags, email_links, has_children).
- Used on startup to hydrate the Svelte store in a single round-trip.

---

#### `create_task`

```
Params:
  input: {
    parent_id?:   string | null   // UUID of parent; null/omitted = root
    caption:      string          // required, 1–500 chars
    note?:        string          // default ""
    position?:    number          // f64; omitted = append after last sibling
    start_date?:  string          // "YYYY-MM-DD" or null
    due_date?:    string          // "YYYY-MM-DD" or null
    reminder_at?: string          // ISO 8601 datetime or null
    flag_id?:     string | null   // UUID; null = no flag
    starred?:     boolean         // default false
    tag_ids?:     string[]        // UUIDs; default []
    is_folder?:   boolean         // default false
    is_project?:  boolean         // default false
  }

Returns: Task   (full object with eager loads)

Errors:
  "caption cannot be empty"
  "caption too long (max 500 chars)"
  "parent_id not found"
  "flag_id not found"
  "tag_id {id} not found"
  "start_date must be YYYY-MM-DD"
  "due_date must be YYYY-MM-DD"
  "due_date cannot be before start_date"
```

- `id` is generated server-side as UUID v4.
- `created_at` and `updated_at` are set to the current UTC timestamp in ISO 8601.
- If `position` is omitted, the backend executes `SELECT MAX(position) FROM tasks WHERE parent_id IS ?` and adds 1.0. If no siblings exist, position = 1.0.
- `tag_ids` rows are inserted into `task_tags` junction table after the task row is created.
- If `is_folder` is true, `is_project` is forced to false (mutually exclusive).

---

#### `update_task`

```
Params:
  id:    string   // UUID of task to update
  input: {
    caption?:          string          // 1–500 chars
    note?:             string
    start_date?:       string | ""     // "" = clear (set to NULL)
    due_date?:         string | ""     // "" = clear
    reminder_at?:      string | ""     // "" = clear
    recurrence_rule?:  string | ""     // JSON blob or "" = clear
    flag_id?:          string | ""     // "" = clear
    starred?:          boolean
    color?:            string | ""     // CSS hex "#RRGGBB" or "" = clear
    is_folder?:        boolean
    is_project?:       boolean
    hide_in_views?:    boolean
    subtasks_in_order?:boolean
    inherit_dates?:    boolean
    custom_format?:    string | ""     // JSON blob or "" = clear
    tag_ids?:          string[]        // replaces entire tag set for this task
  }

Returns: Task   (full updated object)

Errors:
  "task not found"
  "caption cannot be empty"
  "caption too long (max 500 chars)"
  "start_date must be YYYY-MM-DD"
  "due_date must be YYYY-MM-DD"
  "due_date cannot be before start_date"
  "flag_id not found"
  "tag_id {id} not found"
  "recurrence_rule is not valid JSON"
  "custom_format is not valid JSON"
  "color must be #RRGGBB or empty"
```

- Only fields present in `input` are updated; absent fields are not touched (true PATCH semantics).
- `updated_at` is always refreshed on any update.
- `tag_ids` present → delete all rows in `task_tags` for this task, insert new set.
- `tag_ids` absent → tag assignments unchanged.
- `is_folder: true` → `is_project` forced to false in the same statement.
- `is_project: true` → `is_folder` forced to false.
- `recurrence_rule` JSON is validated with `serde_json::from_str` before storing.

---

#### `delete_task`

```
Params:  { id: string }
Returns: ()
Errors:
  "task not found"
```

- Deletes a single task row. Does NOT recursively delete children (use `delete_task_recursive`).
- Due to `ON DELETE CASCADE`, children are promoted: their `parent_id` will be set to NULL by the FK cascade only if the schema uses `ON DELETE SET NULL` for `parent_id`. **Clarification**: the schema uses `ON DELETE CASCADE` on `parent_id` references, meaning children are also deleted. If the intent is orphaning (not cascade), the schema must be changed. Treat `delete_task` as cascade-delete for MVP; expose `delete_task_recursive` as a distinct explicit call from the right-click menu.

---

#### `delete_task_recursive`

```
Params:  { id: string }
Returns: ()
Errors:
  "task not found"
```

- Explicitly deletes the task and all descendants in a single `DELETE` using a recursive CTE:
  ```sql
  WITH RECURSIVE descendants(id) AS (
    SELECT id FROM tasks WHERE id = ?
    UNION ALL
    SELECT t.id FROM tasks t JOIN descendants d ON t.parent_id = d.id
  )
  DELETE FROM tasks WHERE id IN (SELECT id FROM descendants);
  ```
- `task_tags` rows are cleaned up by `ON DELETE CASCADE` on `task_tags.task_id`.

---

#### `complete_task`

```
Params:  { id: string, completed: boolean }
Returns: Task   (updated)
Errors:
  "task not found"
```

- `completed: true` → sets `completed_at = NOW()`.
- `completed: false` → sets `completed_at = NULL`.
- Does not affect children.
- If task has a `recurrence_rule` and `completed = true`, the backend does NOT auto-advance; call `skip_occurrence` separately.

---

#### `complete_branch`

```
Params:  { id: string, completed: boolean }
Returns: ()
Errors:
  "task not found"
```

- Recursively completes or uncompletes the task and all its descendants using a recursive CTE.
- All affected rows get `completed_at = NOW()` (complete) or `NULL` (uncomplete).
- Returns `()` — the frontend must call `get_all_tasks_flat` or update the store manually.

---

#### `move_task`

```
Params:  { id: string, new_parent_id?: string | null, new_position: number }
Returns: Task   (updated)
Errors:
  "task not found"
  "new_parent_id not found"
  "cannot move task into its own descendant"
```

- Updates `parent_id` and `position` in a single statement.
- Cycle check: before writing, verify that `new_parent_id` is not the task itself or any of its descendants (recursive CTE check). Return error if cycle detected.
- `new_position` is a free-form `f64`. Caller is responsible for computing the midpoint value for insertion between two siblings.
- After move, the frontend should call `reorder_tasks` if it wants to normalise positions.

---

#### `reorder_tasks`

```
Params:  { ids_and_positions: [string, number][] }
Returns: ()
Errors:
  "task {id} not found"
```

- Batch-updates `position` for each `(id, position)` pair in a single transaction.
- All IDs must exist; if any does not, the transaction is rolled back and the error is returned.
- Used after drag-and-drop to persist the new order in a single round-trip.

---

#### `duplicate_task`

```
Params:  { id: string }
Returns: Task   (the new copy)
Errors:
  "task not found"
```

- Creates a shallow copy: all scalar fields are duplicated, subtasks are NOT copied.
- New task gets a new UUID v4 `id`.
- `position` = original `position + 0.5` (inserted immediately after original).
- `created_at` and `updated_at` = NOW().
- `completed_at` = NULL (copies are always incomplete).
- Tags are copied (new rows in `task_tags` with the new task id).
- Email links are NOT copied.

---

#### `sort_subtasks`

```
Params:
  parent_id?: string | null   // null = sort root tasks
  sort_by:    string          // "caption" | "due_date" | "start_date" | "created_at" | "flag" | "starred"
  sort_dir:   string          // "asc" | "desc"

Returns: ()

Errors:
  "invalid sort_by value"
  "invalid sort_dir value"
```

- Fetches all direct children of `parent_id`, sorts them in Rust, then writes back sequential `position` values (1.0, 2.0, 3.0 …) in a single transaction.
- `sort_by: "flag"` sorts by the flag's `position` field (which represents user-defined flag priority order).
- `sort_by: "starred"` → starred tasks first.
- NULL date values sort last in both `asc` and `desc`.

---

#### `skip_occurrence`

```
Params:  { id: string }
Returns: Task   (updated, with new dates)
Errors:
  "task not found"
  "task has no recurrence_rule"
  "invalid recurrence_rule JSON"
```

- Reads the `recurrence_rule` JSON blob, advances `start_date` / `due_date` / `reminder_at` to the next occurrence according to the recurrence rule.
- Sets `completed_at = NULL` (marks task as active again for the next cycle).
- Does NOT create a new task row; the same row is reused.
- Recurrence calculation rules: see §16 (Recurrence). All date arithmetic is UTC.

---

#### `get_flags` / `create_flag` / `update_flag` / `delete_flag` / `reorder_flags`

```
create_flag:
  Params:  { name: string, color: string }   // color: "#RRGGBB"
  Returns: Flag
  Errors:  "name cannot be empty", "name too long (max 50 chars)", "color must be #RRGGBB"
           "name already exists"

update_flag:
  Params:  { id: string, name: string, color: string }
  Returns: Flag
  Errors:  "flag not found", same validation as create

delete_flag:
  Params:  { id: string }
  Returns: ()
  Errors:  "flag not found"
  Note:    tasks.flag_id is set to NULL via ON DELETE SET NULL. No cascade delete of tasks.

reorder_flags:
  Params:  { ids_and_positions: [string, number][] }
  Returns: ()
  Errors:  "flag {id} not found"
```

---

#### `get_tags` / `create_tag` / `update_tag` / `delete_tag`

```
create_tag:
  Params:  { name: string, color: string }
  Returns: Tag
  Errors:  "name cannot be empty", "name too long (max 50 chars)", "color must be #RRGGBB"
           "name already exists"

update_tag:
  Params:  { id: string, name: string, color: string }
  Returns: Tag
  Errors:  "tag not found", same validation as create

delete_tag:
  Params:  { id: string }
  Returns: ()
  Errors:  "tag not found"
  Note:    task_tags rows are deleted via ON DELETE CASCADE on tag_id.
```

---

#### `get_views` / `create_view` / `update_view` / `delete_view` / `reorder_views`

```
ViewPayload:
  {
    name:             string        // 1–100 chars
    show_completed:   boolean       // default false
    group_by:         string        // "none" | "flag" | "due_date" | "start_date" | "tag"
    sort_by:          string        // "position" | "caption" | "due_date" | "start_date" | "created_at" | "flag" | "starred"
    sort_dir:         string        // "asc" | "desc"
    visible_fields:   string[]      // subset of: ["flag","tags","due_date","start_date","reminder","note_indicator","email_links"]
    filter_json:      string        // JSON blob — see §5.4
  }

create_view:
  Params:  { payload: ViewPayload }
  Returns: SavedView
  Errors:  "name cannot be empty", "name too long", "invalid group_by", "invalid sort_by",
           "invalid sort_dir", "filter_json is not valid JSON"

update_view:
  Params:  { id: string, payload: ViewPayload }
  Returns: SavedView
  Errors:  "view not found", same validation

delete_view:
  Params:  { id: string }
  Returns: ()
  Errors:  "view not found"

reorder_views:
  Params:  { ids_and_positions: [string, number][] }
  Returns: ()
  Errors:  "view {id} not found"
```

---

#### `add_email_link` / `delete_email_link`

```
add_email_link:
  Params:
    task_id:    string         // UUID
    link_type:  string         // "message_id" | "thread_id" | "mailto"
    link_data:  string         // the raw ID or email address; max 2000 chars
    subject?:   string         // display label; max 500 chars
  Returns: string              // UUID of the new email_link row
  Errors:
    "task not found"
    "invalid link_type"
    "link_data cannot be empty"
    "link_data too long"

delete_email_link:
  Params:  { id: string }
  Returns: ()
  Errors:  "email link not found"
```

- `link_type: "message_id"` — links to a specific Gmail message; opens `https://mail.google.com/mail/u/0/#all/{link_data}`.
- `link_type: "thread_id"` — links to a Gmail thread; opens `https://mail.google.com/mail/u/0/#all/{link_data}`.
- `link_type: "mailto"` — opens the system mail client with `mailto:{link_data}`.
- The URL construction and `shell_open` call happen in the frontend using `tauri-plugin-opener`, not in the backend. The backend only stores and returns the raw data.

---

#### `get_setting` / `set_setting` / `get_all_settings`

```
get_setting:
  Params:  { key: string }
  Returns: string | null       // null if key not set
  Errors:  none

set_setting:
  Params:  { key: string, value: string }
  Returns: ()
  Errors:  "key cannot be empty", "key too long (max 100 chars)", "value too long (max 4000 chars)"
  Note:    Uses INSERT OR REPLACE on the settings table.

get_all_settings:
  Params:  none
  Returns: Record<string, string>    // { key: value, ... }
  Errors:  none (returns {} on DB error)
```

Valid setting keys and value formats — see §5.3 (App Settings Keys). Unrecognised keys are stored without error to allow forward compatibility.

---

#### `gdrive_auth_status`

```
Params:  none
Returns: boolean   // true = token exists and is not expired
Errors:  none
```

- Checks whether a valid GDrive OAuth2 token is stored in the settings table (`gdrive_token` key).
- Does NOT make a network call. Returns false if token is missing, malformed, or expired (by checking expiry timestamp in the token JSON).

---

#### `gdrive_connect`

```
Params:  none
Returns: SyncResult
Errors:
  "OAuth2 flow failed: {reason}"
  "failed to exchange code: {http_status}"
  "failed to store token"
```

```rust
pub struct SyncResult {
    pub success: bool,
    pub message: String,          // human-readable status
    pub conflicts: Vec<SyncConflict>,
    pub uploaded: u32,
    pub downloaded: u32,
}
```

- Opens a loopback HTTP listener on `127.0.0.1:0` (random port), launches the GDrive OAuth2 consent URL in the system browser.
- Waits up to 120 seconds for the redirect with the auth code.
- Exchanges code for token, stores token JSON in settings table.
- `conflicts` is always empty for this command.

---

#### `gdrive_upload`

```
Params:  none
Returns: SyncResult
Errors:
  "not authenticated — call gdrive_connect first"
  "failed to read local DB: {reason}"
  "upload failed: {http_status} {body}"
  "token refresh failed"
```

- Reads the local `tasks.db` file, uploads it to GDrive as a binary file named `taskclaw-backup.db` in the app folder.
- If the file already exists on GDrive, it is replaced (update, not create).
- `uploaded: 1`, `downloaded: 0`, `conflicts: []`.
- Token is refreshed automatically if expired before upload.

---

#### `gdrive_download`

```
Params:  none
Returns: SyncResult
Errors:
  "not authenticated"
  "remote file not found"
  "download failed: {http_status}"
  "failed to write local DB: {reason}"
```

- Downloads `taskclaw-backup.db` from GDrive, overwrites the local `tasks.db`.
- The frontend must call `get_all_tasks_flat` after this to refresh the store.
- `downloaded: 1`, `uploaded: 0`, `conflicts: []`.

---

#### `gdrive_sync`

```
Params:  none
Returns: SyncResult
Errors:
  "not authenticated"
  "download failed: {reason}"
  "merge failed: {reason}"
```

- Full smart sync: see §15 (Sync) for the conflict detection algorithm.
- `conflicts` array is populated when the algorithm detects rows modified on both sides.
- Each conflict:
  ```rust
  pub struct SyncConflict {
      pub task_id: String,
      pub local_caption: String,
      pub remote_caption: String,
      pub local_updated_at: String,
      pub remote_updated_at: String,
      pub resolution: Option<String>,   // "local" | "remote" | null (unresolved)
  }
  ```
- If `conflicts` is non-empty, `success: false` and the frontend must show the conflict resolution screen before committing the merge.
- After the user resolves all conflicts, the frontend calls `gdrive_sync` again with the resolutions applied (mechanism TBD in §15 — likely a separate `gdrive_resolve_conflicts` command with a `Vec<(task_id, resolution)>` param).

---

#### `gdrive_disconnect`

```
Params:  none
Returns: ()
Errors:  none
```

- Deletes `gdrive_token` from the settings table. Does not revoke the token server-side.

---

### 6.6 Error Handling Contract

All commands follow these conventions:

1. **Not found** → return `Err("entity not found")` — never panic.
2. **Validation** → return `Err("field: reason")` before touching the DB.
3. **DB error** → return `Err("db error: {sqlite_error_message}")`.
4. **Network error** (sync only) → return `Err("network: {reason}")`.
5. All error strings are English, lowercase, no trailing period.
6. The frontend displays error strings verbatim in a transient toast notification (red background, 4s auto-dismiss).
7. No command returns HTTP status codes or numeric error codes — string only.

---

### 6.7 Transaction Boundaries

| Command | Transaction scope |
|---|---|
| `create_task` | Single transaction: INSERT tasks + INSERT task_tags |
| `update_task` | Single transaction: UPDATE tasks + DELETE/INSERT task_tags |
| `delete_task` / `delete_task_recursive` | Single statement (cascade handles children) |
| `complete_branch` | Single transaction: UPDATE all matching rows |
| `reorder_tasks` / `reorder_flags` / `reorder_views` | Single transaction: all UPDATEs |
| `duplicate_task` | Single transaction: INSERT tasks + INSERT task_tags |
| `sort_subtasks` | Single transaction: all position UPDATEs |
| `gdrive_sync` | No DB transaction: DB is replaced as a file; WAL checkpoint runs before upload |

---

## 7. Frontend Architecture

### 7.1 Svelte Stores (`src/lib/stores/tasks.ts`)

```typescript
// Core data
export const allTasks    = writable<Task[]>([]);
export const flags       = writable<Flag[]>([]);
export const tags        = writable<Tag[]>([]);
export const views       = writable<SavedView[]>([]);

// Derived
export const taskById    = derived(allTasks, ts => new Map(ts.map(t => [t.id, t])));
export const rootTasks   = derived(allTasks, ts => ts.filter(t => !t.parent_id && !t.completed_at).sort(...));
export const searchQuery = writable<string>('');

// UI state
export const activeTabId       = writable<string>('outline');
export const selected          = writable<Set<string>>(new Set());  // multi-select support
export const detailTaskId      = writable<string | null>(null);     // shown in Task Detail panel
export const editingId         = writable<string | null>(null);     // inline edit
export const expanded          = writable<Set<string>>(new Set());
export const showPrefs         = writable<boolean>(false);
export const showRapidInput    = writable<boolean>(false);
export const outlineScrollToId = writable<string | null>(null);
export const lastUsedFlagId    = writable<string | null>(null);     // for single-click flag toggle
export const contextMenu       = writable<ContextMenuState | null>(null);

// Functions (exported)
export async function loadAll(): Promise<void>
export async function createTask(input): Promise<Task>
export async function updateTask(id, input): Promise<Task>
export async function deleteTask(id): Promise<void>
export function setSelected(id: string, multi: boolean): void
export function expandToTask(id: string): void
export function navigateToOutline(id: string): void
export function expandAll(): void
export function collapseAll(): void
export function clearSelection(): void
```

### 7.2 Component Tree

```
+page.svelte
├── <header class="titlebar">
│   ├── AppName
│   ├── TabBar (Outline + one per view)
│   └── TitlebarRight
│       ├── <button ⚙> → showPrefs
│       └── SyncBar
├── <div class="main-area">
│   ├── <div class="content">
│   │   ├── TaskTree          (activeTabId === 'outline')
│   │   └── GroupedView       (activeTabId === view.id)
│   ├── TaskDetail            (shown when detailTaskId !== null)
│   └── ViewsPanel            (slide-in, always mounted)
├── Prefs                     (modal, mounted when showPrefs)
├── ReminderWindow            (floating, always mounted)
├── RapidInput                (modal, mounted when showRapidInput)
└── ContextMenu               (portal, mounted when contextMenu !== null)
```

### 7.3 Routing

SvelteKit with SSR disabled. Single route: `src/routes/+page.svelte`.
`src/routes/+layout.ts` must contain `export const ssr = false; export const prerender = false;`

---

## 8. Application Layout

### 8.1 Titlebar (height: 36px)

```
[TaskClaw]  [☰ Outline] [View 1] [View 2]  ···  [⚙]  [↑ 12:04 ↓]
```

- `-webkit-app-region: drag` on titlebar; `no-drag` on interactive elements
- **App name**: monospace, 12px, accent colour, `flex-shrink: 0`
- **Tab bar**: `flex: 1`, `overflow-x: auto`, hidden scrollbar
  - Each tab: 12px, `padding: 0 14px`, `border-top: 2px solid transparent`
  - Active tab: `border-top-color: var(--accent)`, `color: var(--accent)`, `background: var(--bg)`
  - Hover tab: `color: var(--text)`, `background: var(--hover)`
- **⚙ button**: icon button, 14px, opens Preferences modal
- **SyncBar**: see §15

### 8.2 Main Area

Below titlebar, `flex: 1`, `overflow: hidden`.

```
[Content area (flex: 1)]  |  [Task Detail (320px, resizable)]
```

- Task Detail panel shown only when `$detailTaskId` is non-null
- Divider between content and detail is a draggable resize handle (min content: 400px, min detail: 240px)
- ViewsPanel slides in over the right edge (width: 260px, `position: absolute; right: 0`)

---

## 9. Task Tree (Outline Tab)

### 9.1 Toolbar

Height 32px, `background: var(--surface-elevated)`, border-bottom.

```
[+ Task]  [📋 Rapid]  [Search…]  ·····  [⊞] [⊟] [✕]
```

- **+ Task**: creates a root task, immediately enters inline edit
- **📋 Rapid**: opens Rapid Input modal
- **Search**: `width: 180px`; filters task tree in real time (caption contains, case-insensitive); shows all ancestors of matching tasks
- **⊞**: expand all; **⊟**: collapse all; **✕**: clear selection

### 9.2 Column Headers

Height 22px, `background: var(--surface-elevated)`, border-bottom.

```
[56px spacer]  [Task — flex:1]  [Start — 72px]  [Due — 72px]
```

The 56px spacer accounts for: expand toggle (14px) + checkbox (18px) + flag dot (12px) + gaps.

### 9.3 TaskRow Component

Props: `task: Task`, `depth: number`, `siblings: Task[]`

#### Layout (28px height)

```
[indent: depth×16px] [▶/▼ 14px] [☐ 18px] [● 12px] [caption flex:1] [start 72px] [due 72px]
```

- **Indent**: `padding-left: depth * 16px`
- **Expand toggle**: `▶` when collapsed/has children; `▼` when expanded; invisible but space reserved when leaf
- **Checkbox**: square, 14px, checked = `completed_at` set
- **Flag dot**: 8px circle, `background: flag.color`; if no flag, space still reserved (12px)
- **Caption**: `flex: 1`, `overflow: hidden`, `text-overflow: ellipsis`, `white-space: nowrap`
  - If `is_folder`: show folder icon prefix `📁`
  - If `is_project`: bold
  - If `starred`: show `⭐` suffix
  - Custom format applied if set (bold/italic/colour)
- **Start / Due**: right-aligned, 11px, colour-coded:
  - Overdue (date < today): `var(--red)`
  - Today: `var(--amber)`
  - Future / no date: `var(--text-dim)`
  - Format: `Mar 10` (no year if same year), `Mar 10 14:30` if has time

#### Click Behaviour

| Target | Action |
|---|---|
| Caption area | Enter inline edit mode (click again if already selected) |
| Flag dot | If flag set → clear flag. If no flag → apply `$lastUsedFlagId` (or first flag if none). Update `lastUsedFlagId`. |
| Star suffix / star hover button | Toggle `starred`. |
| Checkbox | Toggle `completed_at`. If `subtasks_in_order`, validate previous subtask is complete. |
| Expand toggle | Toggle expanded state. |
| Anywhere else on row | Select task: `setSelected(id, e.ctrlKey || e.metaKey)` |

Selection (`setSelected`):
- Single click (no modifier): clear existing selection, select this task, set `detailTaskId = id`
- Ctrl/Cmd+click: add/remove from selection (multi-select), `detailTaskId` = last clicked
- Shift+click: range select (select all tasks between last selected and this one in visible order)

#### Hover Actions (appear on row hover, right side)

```
[✎ edit]  [+ child]  [⭐ star]  [🗑 delete]
```

- **✎**: enter inline edit
- **+ child**: create subtask immediately below, enter inline edit
- **⭐**: toggle star
- **🗑**: delete (with confirm dialog if task has subtasks)

#### Inline Edit Mode

- Caption becomes a text `<input>` in place
- **Enter**: save, exit edit
- **Alt+Enter**: apply NLP parsing (§20), save, exit edit
- **Escape**: discard, exit edit
- **Tab**: save, create next sibling task, enter its edit mode
- **Shift+Tab**: save, exit edit, move focus to parent

#### Drag and Drop Reorder

- Drag handle: the flag dot area (cursor: grab)
- Drop indicator: horizontal line between rows
- Can reorder within same parent
- Drop onto a task (centre zone): reparent as last child
- Drop between tasks: insert at that position
- After drop: call `reorder_tasks` or `move_task` as appropriate

#### Context Menu

Right-click anywhere on row → see §12.

#### Scroll Flash

When `$outlineScrollToId === task.id`:
1. `scrollIntoView({ block: 'center', behavior: 'smooth' })`
2. Add CSS class `scroll-flash` for 1200ms then remove
3. `outlineScrollToId.set(null)`

```css
@keyframes flash-highlight {
  0%   { background: var(--accent-dim); }
  70%  { background: var(--accent-dim); }
  100% { background: transparent; }
}
.scroll-flash { animation: flash-highlight 1.2s ease-out forwards; }
```

### 9.4 Task Children Loading

All tasks are loaded flat via `get_all_tasks_flat` on startup and stored in `allTasks`.
Hierarchy is derived client-side:

```typescript
export const childrenOf = derived(allTasks, ts => {
  const map = new Map<string | null, Task[]>();
  for (const t of ts) {
    const key = t.parent_id ?? null;
    if (!map.has(key)) map.set(key, []);
    map.get(key)!.push(t);
  }
  // sort each group by position
  for (const arr of map.values()) arr.sort((a, b) => a.position - b.position);
  return map;
});
```

---

## 10. Grouped View Tab

### 10.1 Structure

Each Saved View renders as a grouped list.

```
▼ Group Header (e.g. "🔴 Urgent")
  [task row] [task row] ...
▶ Group Header (collapsed)
▼ Group Header
  ...
```

### 10.2 Group Keys

| `group_by` | Groups |
|---|---|
| `none` | Single group, no header shown |
| `flag` | One group per flag + one "No Flag" group |
| `tag` | One group per tag + "No Tag". Tasks in multiple tags appear in each. |
| `due` | Overdue / Today / This week / Later / No due date |
| `start` | Same buckets as `due` but by `start_date` |

### 10.3 Columns

Columns are controlled by `view.visible_fields`. Possible values:

`caption` (always shown) | `flag` | `tags` | `start_date` | `due_date` | `reminder_at` | `starred` | `note_preview`

Column widths: caption = flex 1; all others = fixed (72px for dates, 48px for icons, 140px for note preview).

### 10.4 Row Interactions

| Action | Behaviour |
|---|---|
| Single click (row) | Select task, show in Task Detail |
| Double click (row) | `navigateToOutline(task.id)` |
| Right click | Context menu (§12) |
| Click flag dot | Toggle flag (same as Outline) |
| Click star | Toggle star |

No inline editing in Grouped View. Double-click navigates to Outline for editing.

---

## 11. Task Detail Panel

Width: 320px (resizable, min 240px, max 480px). Shown when a task is selected.

Panel has a header and a scrollable body. Body contains sections.

### 11.1 Panel Header

```
[Caption — editable h2]                          [✕ close]
```

Caption is an editable `<h2>` (`contenteditable`). Saves on blur or Enter. Alt+Enter applies parsing.

### 11.2 Notes Section (fixed, always first, cannot be hidden)

Markdown-capable text editor.

- Default state: render markdown (read-only display)
- On click: switch to edit mode (plain textarea with toolbar)
- Toolbar buttons (shown in edit mode):
  - **B** (Ctrl+B) — `**bold**`
  - *I* (Ctrl+I) — `*italic*`
  - U̲ (Ctrl+U) — `<u>underline</u>`
  - ~~S~~ — `~~strikethrough~~`
  - Font colour picker → inserts `<span style="color:…">…</span>`
  - Bullet list — inserts `- ` prefix
  - Link — prompts for URL, inserts `[text](url)`
  - Image — prompts for URL, inserts `![alt](url)`
- Save on blur (outside textarea)
- Preview uses a lightweight markdown parser (no external library required — implement basic: headers, bold, italic, bullets, links; skip tables/code)

### 11.3 Collapsible Sections

Each section:
- Header bar: `[▶/▼] [Section Name]` left; `[⠿ drag handle]` right
- Click header → collapse/expand (animated: max-height transition 150ms)
- Drag handle → reorder sections (drag-and-drop)
- User can hide sections in Preferences → Task Detail. Hidden sections don't render at all.
- Order and visibility persisted in `app_settings` as `detail_panel_order` and `detail_panel_hidden`

#### Section: General

```
☐ Folder              (is_folder)
☐ Project             (is_project)
☐ Hide in Views       (hide_in_views)
☐ Complete in Order   (subtasks_in_order)

Tags: [chip][chip][+ add tag…]
```

- Each tag chip: coloured pill with ✕ to remove
- "add tag…": text input with autocomplete dropdown showing existing tags. Enter creates new tag if not found.

#### Section: Timing & Reminder

```
☐ Inherit parent dates

Today | Next day | Next week          ← quick-set hyperlinks for Start date

Start:  [date input]  ☐ Use time  [time input]
Due:    [date input]  ☐ Use time  [time input]

Reminder: ☐  [date input] [time input]  [✕ clear]

[↺ Recurrence…]
```

- **Inherit parent dates**: when checked, `start_date` and `due_date` are read-only and show parent's values greyed out
- **Quick-set links** ("Today", "Next day", "Next week") apply to the Start date field
- **Use time**: single checkbox controls both Start and Due time inputs. When unchecked, time inputs hidden and time component stripped from stored value
- **Reminder checkbox**: when checked, shows date+time fields. When unchecked, clears `reminder_at`
- **Auto-fill rule**: when Start date is set and Due date is empty → auto-fill Due = Start. When Start is set and Reminder is empty → auto-fill Reminder = Start datetime
- **Recurrence link**: opens Recurrence modal (§17)

Date inputs: native `<input type="date">` with custom styling.
Time inputs: native `<input type="time">` (24h format internally, display per OS locale).

#### Section: Format

```
☐ Use custom formatting

[B] [I] [U] [S̶] [A▾ highlight] [A▾ font color]  [▌sidebar color]
☐ Subtasks inherit format
```

- All controls disabled when "Use custom formatting" is unchecked
- Colour pickers: `<input type="color">` styled as small square
- Changes apply immediately to task row and propagate to subtasks if "inherit" is checked
- Sidebar colour: renders as a 3px left border on the task row in the outline

---

## 12. Right-Click Context Menu

Triggered by right-click on any task row (Outline or Grouped View).

### 12.1 Component: `ContextMenu.svelte`

- Rendered as a fixed-position overlay `<div>` portaled to `<body>`
- Positioned at mouse coordinates; repositioned if it would overflow viewport
- Dismissed by: click outside, Escape, scroll
- Submenus open on hover (300ms delay) to the right; flip left if near right edge

### 12.2 Menu Definition

```
New Task                          → createTask({ parent_id: null }) at same level
New Subtask                       → createTask({ parent_id: task.id })
New Project                       → createTask({ is_project: true, parent_id: ... })
New Folder                        → createTask({ is_folder: true, parent_id: ... })
──────────────────────────────────
Set Start Date        ▶           [date submenu — see §12.3]
Set Due Date          ▶           [date submenu — see §12.3]
Skip Occurrence                   [only if recurrence_rule set] → skip_occurrence(id)
──────────────────────────────────
Cut                               → store in clipboard store; Cut = copy + mark for deletion on paste
Copy                              → store task data in clipboard store
Copy as Local Link                → copy "taskclaw://task/<id>" to system clipboard
Copy as URL                       → copy "taskclaw://task/<id>" to system clipboard (same for now)
Duplicate Task                    → duplicate_task(id) → new task appears below
Move To…                          → opens task-picker modal (§12.4)
Paste                             [shown only if clipboard has content]
──────────────────────────────────
Advanced                ▶
  Complete Task and All Subtasks  → complete_branch(id, true)
  Uncomplete Task and All Subtasks→ complete_branch(id, false)
  ────────────────────────────────
  Sort Subtasks…                  → opens sort dialog (§12.5)
  ────────────────────────────────
  Copy Tasks as Text              → copies caption tree as indented plain text to clipboard
──────────────────────────────────
Delete Task                       → delete_task(id) [confirm if has subtasks]
──────────────────────────────────
Tag                     ▶         [tag submenu — see §12.6]
Flag                    ▶         [flag submenu — see §12.7]
Star                    ▶
  Star Task                       → updateTask(id, { starred: true })
  Clear Star                      → updateTask(id, { starred: false })
  Toggle Star                     → updateTask(id, { starred: !task.starred })
```

### 12.3 Date Submenu

Used for both "Set Start Date" and "Set Due Date".
The submenu dynamically generates "In N days" entries for the next 6 days relative to today.

```
Calendar…                         → opens a small inline monthly calendar popover
──────────────────────────────────
Today          Tue Mar 10
Tomorrow       Wed Mar 11
In 2 days      Thu Mar 12
In 3 days      Fri Mar 13
In 4 days      Sat Mar 14
In 5 days      Sun Mar 15
In 6 days      Mon Mar 16
──────────────────────────────────
Set Equal to Due Date              (shown in Start submenu only)
Set Equal to Start Date            (shown in Due submenu only)
Next Day                          → date + 1
Next Week                         → date + 7
Previous Day                      → date - 1
──────────────────────────────────
Clear
```

**Calendar popover**: 6×7 grid, month navigation (< >) arrows, today highlighted, click a day to set.

### 12.4 Move To… Modal

- Title: "Move Task To…"
- Search input (auto-focused)
- Scrollable list of all tasks rendered as indented tree (same indentation style as Rapid Input preview)
- Folder/project tasks shown with distinct style
- Click a task → `move_task(taskId, targetId, newPosition)` → close modal
- "Move to root" option at top
- Keyboard: arrow keys navigate list, Enter confirms, Escape cancels

### 12.5 Sort Subtasks Dialog

Small modal:
```
Sort subtasks of "[parent caption]"

Sort by: ○ Name  ○ Due Date  ○ Start Date  ○ Created Date
Order:   ○ Ascending  ○ Descending

[Cancel]  [Sort]
```

Calls `sort_subtasks(parent_id, sort_by, sort_dir)`.

### 12.6 Tag Submenu

```
[✓] Tag Name 1     ← checkmark if task has this tag
    Tag Name 2
    Tag Name 3
──────────────────
    Clear All Tags
```

Click a tag → toggle it on/off for this task.

### 12.7 Flag Submenu

```
[●] 🔴 Urgent      ← dot shown if this is the current flag
[●] 🟡 Review
[●] 🔵 Waiting
[●] 🟢 Delegated
──────────────────
    Clear Flag
```

Click a flag → set as task's flag (replacing any existing). If already set → clear it.

---

## 13. Rapid Input

### 13.1 Trigger

- Button "📋 Rapid" in Outline toolbar
- Keyboard: `Ctrl+Shift+I` (global, from `svelte:window`)

### 13.2 Layout

Full-screen modal (max-width: 900px, centered, 80vh height).

```
┌──────────────────────────────────────────────────────────────┐
│ Rapid Input                                         [✕ close] │
│ Parent: [task picker dropdown ▾]                             │
│ ☑ Apply parsing                                              │
├──────────────────────┬───────────────────────────────────────┤
│ Paste tasks here…    │  Preview                              │
│ (textarea)           │  ├── Task 1         d:Mar 15  🔵      │
│                      │  │   ├── Subtask 1a                   │
│                      │  │   └── Subtask 1b  s:Mar 10         │
│                      │  └── Task 2         ⭐                │
├──────────────────────┴───────────────────────────────────────┤
│ Syntax: !Flag  #tag  *star  s:date  d:date  remind N min    │
│                              [Cancel]  [Import  Ctrl+Enter]  │
└──────────────────────────────────────────────────────────────┘
```

### 13.3 Parent Picker

Dropdown with search. Shows full task hierarchy with indentation prefix (`│ `, `├─ `, `└─ `). Selecting a task makes all imported tasks children of that task. Default: "Root (no parent)".

### 13.4 Indentation Detection

1. Scan all lines for indentation characters
2. If tabs found: tab = 1 level
3. If spaces found: detect consistent unit (smallest non-zero indent = 1 level)
4. Mixed: normalise (tab = 4 spaces)

### 13.5 Live Preview

Updates on every keystroke (debounced 100ms).
Shows the parsed task tree with coloured badges:
- Flag: coloured dot `●`
- Tags: coloured pill chips
- Start date: green badge
- Due date: red badge
- Reminder: gold badge
- Star: `⭐`

### 13.6 Import

`Ctrl+Enter` or "Import" button:
1. Parse all lines into tree structure
2. Create tasks recursively (parent first, then children) via `create_task`
3. If parent picker has a selection, set `parent_id` accordingly
4. On complete: close modal, navigate to Outline, scroll to first created task

---

## 14. Inline Parsing (Alt+Enter)

Applied in two places: TaskRow inline edit (Alt+Enter) and Rapid Input (when "Apply parsing" is on).

### 14.1 Parser Input/Output

Input: raw string
Output:
```typescript
{
  caption: string,
  flagId: string | null,       // matched by name, fuzzy
  tagIds: string[],
  starred: boolean,
  startDate: string | null,    // YYYY-MM-DD or YYYY-MM-DDTHH:mm
  dueDate: string | null,
  reminderAt: string | null,   // always with time
}
```

### 14.2 Parse Order (left-to-right tokens, then NLP from right)

1. Extract quoted caption: `"quoted text"` → protected from further parsing
2. Extract explicit tokens (anywhere in string):
   - `!FlagName` → flag by name (case-insensitive, fuzzy)
   - `#tagname` or `@tagname` → tag
   - `*` (standalone) → starred
   - `-star` or `-*` → starred
   - `-fl<name>` → flag by name
   - `-s` → next date expr is start
   - `-d` → next date expr is due
   - `s:<expr>` → start date
   - `d:<expr>` → due date
3. Find `remind`/`rmd` keyword → extract remainder as reminder expression
4. Apply NLP date detection on remaining words (right-to-left boundary scan)
5. Remainder after all extraction = caption

### 14.3 Date Expression Formats

| Input | Result |
|---|---|
| `today` | today 00:00 |
| `tomorrow` | today + 1 day |
| `monday` … `sunday` | next occurrence of that weekday |
| `next monday` | next occurrence (always in future, ≥ 1 day) |
| `in N days` | today + N |
| `in N weeks` | today + N×7 |
| `in N months` | today + N months |
| `jan 26` / `january 26` | Jan 26 of current or next year (whichever is future) |
| `3/15` | March 15 |
| `2026-01-15` | ISO date |
| `+Nd` | today + N days |
| `+Nw` | today + N weeks |
| Time suffix: `3pm`, `15:30`, `at 3pm` | Set time component |
| `in 3 days 4pm` | Combined relative + time |
| `next friday 9am` | Combined weekday + time |

### 14.4 Reminder Expressions

After `remind` or `rmd` keyword:

| Input | Result |
|---|---|
| `10 min in advance` / `10 min before` | due_datetime − 10 min |
| `1 hour before` | due_datetime − 1 hour |
| `me` | same as due_datetime |
| `me tomorrow 9am` | absolute datetime |
| `me [any date expr]` | absolute datetime |

If due date has no time, reminder offset applies to 00:00 of due date.

---

## 15. Sync (GDrive)

### 15.1 Architecture

The entire `Data/tasks.db` file is synced as a single binary file to a dedicated GDrive folder: `TaskClaw/tasks.db`.

Conflict detection: compare `updated_at` of the most recently modified task locally vs the GDrive file's `modifiedTime` metadata.

### 15.2 SyncBar Component

Displayed in the titlebar right section.

States:
- **Idle, never synced**: `[↑↓ Connect GDrive]`
- **Connected, in sync**: `[↑↓ 12:04]` (last sync time; click = manual sync)
- **Syncing**: `[↑↓ Syncing…]` (animated spinner character)
- **Error**: `[↑↓ ⚠ Error]` (hover shows error tooltip)
- **Conflict**: `[↑↓ ⚠ Conflict]` (click opens Conflict Resolution modal)

### 15.3 Auto-Sync

When `sync_auto_enabled = true`:
- A `setInterval` in `+page.svelte` fires every `sync_interval_min` minutes
- Calls `gdrive_sync` command
- If result has `conflict: true`, show the Conflict Resolution modal
- If result is success, update `sync_last_at` in settings and update SyncBar display

### 15.4 gdrive_sync Logic (Rust)

```
1. Check if GDrive token exists → if not, return error "not connected"
2. Get remote file metadata (modifiedTime)
3. Get local last_sync_at from app_settings
4. If remote modifiedTime > last_sync_at AND local has changes since last_sync_at:
   → return { conflict: true }
5. If remote modifiedTime > last_sync_at (only remote changed):
   → download remote file, replace local db, reload all data
6. If local has changes since last_sync_at (only local changed):
   → upload local db to remote
7. If neither changed:
   → no-op, return { conflict: false, synced: false }
8. Update last_sync_at
```

### 15.5 Conflict Resolution Modal

Shown when sync returns `conflict: true`.

```
┌─────────────────────────────────────────────────────┐
│ Sync Conflict                                        │
│                                                      │
│ Both local and remote data have changed since the    │
│ last sync on [timestamp].                            │
│                                                      │
│ Local:   last change [timestamp], [N] tasks          │
│ Remote:  modified [timestamp]                        │
│                                                      │
│ What would you like to do?                           │
│                                                      │
│ [Keep Local — overwrite remote]                      │
│ [Keep Remote — overwrite local]                      │
│ [Cancel — resolve later]                             │
└─────────────────────────────────────────────────────┘
```

- **Keep Local**: call `gdrive_upload`, update `sync_last_at`
- **Keep Remote**: call `gdrive_download`, reload all data, update `sync_last_at`
- **Cancel**: dismiss modal; conflict badge remains in SyncBar

### 15.6 Preferences → Sync Tab

```
GDrive Connection
  Status: ● Connected as user@gmail.com    [Disconnect]
  (or)
  Status: ○ Not connected                  [Connect…]

Auto-Sync
  ☑ Enable automatic sync
  Sync every: [15 ▾] minutes

Manual Sync
  [↑ Push to GDrive]  [↓ Pull from GDrive]

  ⚠ Force push overwrites remote. Force pull overwrites local.
  Last synced: Tue Mar 10 at 12:04
```

---

## 16. Recurrence

### 16.1 Field Glossary

| Field | Type | Meaning |
|---|---|---|
| `pattern` | string | `"hourly"` \| `"daily"` \| `"weekly"` \| `"monthly"` \| `"yearly"` |
| `interval` | integer ≥ 1 | Recur every N units of `pattern`. E.g. `pattern=weekly, interval=2` = fortnightly. |
| `days_of_week` | int[] | For `pattern=weekly` only. 0=Mon … 6=Sun. At least one day must be set. Ignored for other patterns. |
| `day_of_month` | int \| null | For `pattern=monthly` only. 1–31. If null, uses the day from the current `start_date`. If the value exceeds the days in a given month (e.g. 31 in February), use the last day of that month. |
| `regenerate` | bool | If true: the "interval" is measured from the **completion date**, not from the previous occurrence date. Only relevant for `complete_task` trigger, not for calendar-based patterns. |
| `start_date` | ISO datetime | The reference start datetime for the recurrence series (the first occurrence's start). Used to compute future occurrences. |
| `due_date` | ISO datetime | Reference due datetime. The gap (`due_date − start_date`) is the **period** and is preserved on every advance. |
| `lead_time_days` | integer ≥ 0 | The task row's `start_date` is set to `next_due_date − lead_time_days`. Allows the task to appear in views N days before it is actually due. Set to 0 to disable. |
| `use_time` | bool | If false: store only date part (`YYYY-MM-DD`) in `tasks.start_date` and `tasks.due_date`. If true: store full datetime including time. |
| `lock_period` | bool | If true: when advancing, keep `due_date − start_date` constant (the period is locked). If false: only advance `due_date`; `start_date` stays pinned unless `lead_time_days` > 0. |
| `end_mode` | string | `"none"` = never ends. `"after_n"` = ends after N completions. `"by_date"` = ends on or before a fixed date. |
| `end_after_n` | int \| null | Required when `end_mode = "after_n"`. |
| `end_by_date` | ISO date \| null | Required when `end_mode = "by_date"`. |
| `occurrences_completed` | integer | Incremented each time the task is completed or `skip_occurrence` is called. |
| `advanced.subtask_reset` | string | `"none"` = leave subtasks as-is. `"reset_all"` = mark all direct subtasks uncompleted on advance. `"reset_if_all_complete"` = only reset if every subtask is already completed. |
| `advanced.auto_recur` | string | `"disabled"` = recurrence only advances via explicit `complete_task` or `skip_occurrence`. `"on_any_subtask"` = advance when any direct subtask is completed. `"on_all_subtasks"` = advance when all subtasks are completed. |
| `advanced.no_completed_copy` | bool | If true: do NOT mark the current row as `completed_at`; just advance dates in place. If false: first set `completed_at = NOW()`, then reset. |

**`hourly` pattern note:** applies only to the time component of `start_date`/`due_date`. `use_time` must be true. Suitable for very frequent short tasks (e.g. medication). Interval = every N hours.

---

### 16.2 Next-Occurrence Calculation Algorithm

This is the canonical algorithm used by both `complete_task` (when recurrence applies) and `skip_occurrence`.

```
Input:
  rule       — the RecurrenceRule struct
  ref_date   — the reference date for the advance:
               if rule.regenerate = true  → use today (completion date)
               if rule.regenerate = false → use rule.start_date (calendar-based)

Compute next_start:

  if pattern = "hourly":
    next_start = ref_date + interval hours
    (keep minutes/seconds from ref_date)

  if pattern = "daily":
    next_start = ref_date + interval days

  if pattern = "weekly":
    Find the next calendar day after ref_date that falls on one of days_of_week,
    counting only every `interval`-th week from the series origin.
    Algorithm:
      candidate = ref_date + 1 day
      while true:
        if candidate.weekday ∈ days_of_week
          AND weeks_since_origin(candidate) % interval == 0:
          next_start = candidate
          break
        candidate += 1 day
    weeks_since_origin = floor((candidate − rule.start_date) / 7) / interval,
    aligned to the origin week.

  if pattern = "monthly":
    target_day = rule.day_of_month ?? ref_date.day
    next_month = ref_date.month + interval  (roll year if > 12)
    next_start = date(next_month.year, next_month.month, min(target_day, days_in(next_month)))

  if pattern = "yearly":
    next_start = date(ref_date.year + interval, ref_date.month, ref_date.day)
    (Feb 29 → Feb 28 on non-leap years)

Compute next_due:
  if lock_period:
    period = rule.due_date − rule.start_date   (duration)
    next_due = next_start + period
  else:
    advance_delta = next_start − rule.start_date
    next_due = rule.due_date + advance_delta

Compute task dates:
  if lead_time_days > 0:
    task.start_date = next_due − lead_time_days days
  else:
    task.start_date = next_start

  task.due_date = next_due

  if task.reminder_at is set:
    reminder_offset = original_rule.due_date − original_task.reminder_at
    task.reminder_at = next_due − reminder_offset

Update rule:
  rule.start_date = next_start
  rule.due_date   = next_due
  occurrences_completed += 1

Check end condition:
  if end_mode = "after_n" AND occurrences_completed >= end_after_n:
    clear recurrence_rule from task (set to NULL)
    do not advance — task stays completed

  if end_mode = "by_date" AND next_start.date > end_by_date:
    clear recurrence_rule from task
    do not advance — task stays completed
```

---

### 16.3 Data Flow: complete_task

When `complete_task(id, completed: true)` is called on a task with a `recurrence_rule`:

1. Parse `recurrence_rule` JSON.
2. Check end condition with current `occurrences_completed` + 1. If series is over → mark `completed_at = NOW()`, clear `recurrence_rule`, return.
3. If `advanced.no_completed_copy = true` → do NOT set `completed_at`.
4. If `advanced.no_completed_copy = false` → set `completed_at = NOW()`.
5. If `advanced.auto_recur = "disabled"` → stop here (do not advance dates — user must call `skip_occurrence` manually).
6. Otherwise → run Next-Occurrence Algorithm, write new `start_date`, `due_date`, `reminder_at`, updated `recurrence_rule` JSON back to the same row, clear `completed_at`.
7. If `advanced.subtask_reset ≠ "none"` → reset subtasks as specified.

---

### 16.4 skip_occurrence Command

Advances to the next occurrence **without** marking the task complete. Intended for the "Skip Occurrence" right-click menu item.

1. Parse `recurrence_rule`.
2. Check end condition. If series over → clear `recurrence_rule`, return task as-is (completed_at stays NULL).
3. Run Next-Occurrence Algorithm with `ref_date = today` (always uses today, regardless of `regenerate` flag).
4. Write new dates and updated rule JSON. Do NOT touch `completed_at`.

---

### 16.5 Edge Cases

| Scenario | Behaviour |
|---|---|
| `days_of_week` is empty on weekly pattern | Reject with error: "weekly recurrence requires at least one day selected". Do not save. |
| `interval` = 0 | Reject: "interval must be at least 1". |
| `end_by_date` is in the past when saving | Accept (no validation). The series will immediately terminate on the first complete/skip. |
| `occurrences_completed >= end_after_n` already | Clear rule immediately on next complete/skip; do not advance. |
| `day_of_month = 31` for a month with 30 days | Use last day of month (30). |
| `day_of_month = 29` in non-leap February | Use 28. |
| `start_date` not set on task when completing | Use today as `ref_date`. |
| `due_date` not set, `lock_period = true` | Treat period as 0 days (`next_due = next_start`). |
| Subtask reset with nested subtasks | Reset applies to direct children only (one level). |

---

### 16.6 Recurrence Modal Layout

Full-width modal, two-panel layout (left: config, right: preview of next 5 occurrences).

**Left panel:**

```
Recurrence Pattern
  ○ Hourly
  ○ Daily
  ● Weekly
  ○ Monthly
  ○ Yearly

Weekly config:
  Recur every [1] week(s) on:
  [Mon] [Tue] [✓Wed] [Thu] [Fri] [Sat] [Sun]

─── OR ───────────────────────────────────────
  ○ Regenerate new task [1] week(s) after each
    task is completed

Next Occurrence
  Start: [date] [time]
  Due:   [date] [time]
  Lead time: [0] days
  ☐ Use time    ☐ Lock period

End Occurrences
  ● No end date
  ○ End after [10] occurrences
  ○ End by [date]

[Cancel]  [Remove Recurrence]  [Advanced…]  [OK]
```

**Right panel:** "Next occurrences" — list of next 5 start/due date pairs computed from the rule.

### 16.7 Advanced Options Modal

```
Subtask reset on recurrence
  ○ Disable automatic reset
  ● Reset all subtasks to uncompleted
  ○ Reset if all subtasks are completed

Auto-recur behaviour
  ● Disable automatic recurrence
  ○ Recur when any subtask is completed
  ○ Recur when all subtasks are completed

☑ Do not create a completed copy on recur

[Cancel]  [Restore Default]  [OK]
```

---

## 17. Preferences Modal

Tabs: **Flags** | **Tags** | **Task Detail** | **Sync** | **Security**

### Flags Tab

```
[● #E05C5C]  🔴 Urgent     [✎] [✕]     ← drag to reorder (⠿ handle)
[● #D4A843]  🟡 Review     [✎] [✕]
[● #4A9EFF]  🔵 Waiting    [✎] [✕]
[● #6ABF69]  🟢 Delegated  [✎] [✕]

[color picker]  [Flag name…]  [+ Add Flag]
```

Edit inline (click ✎). Delete shows confirm if tasks use this flag.

### Tags Tab

```
[#4A9EFF pill: design]  [✕]
[#D4A843 pill: urgent]  [✕]

[color picker]  [Tag name…]  [+ Add Tag]
```

### Task Detail Tab

```
Configure sections shown in the Task Detail panel.
Drag to reorder. Toggle to show/hide.

[⠿] ☑ Notes        (fixed — cannot be hidden)
[⠿] ☑ General
[⠿] ☑ Timing & Reminder
[⠿] ☑ Format
```

Changes saved immediately to `app_settings`.

### Sync Tab

See §15.6.

### Security Tab

```
Database Encryption

Status: 🔓 Not encrypted
        (or 🔒 Encrypted with AES-256)

[Set Password]  /  [Change Password]  [Remove Password]

Database file:
  <app folder>\Data\tasks.db

⚠ Note: Encryption requires closing and reopening the app.
```

Set/Change: prompt for new password + confirm. Validate match before proceeding.
Remove: single confirmation dialog.
Implementation: parked until SQLCipher Windows crash is resolved (see §21).

---

## 18. Reminder Window

### 18.1 Behaviour

- Mounted always in `+page.svelte`, hidden when no active reminders
- Visible when `reminder_at <= now` and task is not dismissed/snoozed
- Re-evaluated every 30 seconds via `setInterval`
- Persisted state in `localStorage` key `tc_reminders`:
  ```json
  { "dismissed": ["taskId1", "taskId2"], "snoozed": { "taskId3": "2026-03-10T14:30" } }
  ```

### 18.2 Layout

Floating fixed panel. Position: `top: 52px; left: 50%; transform: translateX(-50%)`. `z-index: 500`.

```
┌───────────────────────────────────────────────────────────┐
│ Reminders                                       [✕ close] │
├────────────────────┬──────┬────────┬────┬─────────────────┤
│ Task               │ Flag │ Due in │ ⭐  │ Actions         │
├────────────────────┼──────┼────────┼────┼─────────────────┤
│ Write report       │  🔴  │ 2h     │    │[Open][✓][✕][▾] │
│ Call dentist       │      │ Overdue│ ⭐  │[Open][✓][✕][▾] │
└────────────────────┴──────┴────────┴────┴─────────────────┘
                                   Snooze: [5m ▾] [Snooze]
```

### 18.3 Column Resizing

Columns: Task (flex 1) | Flag (48px) | Due in (72px) | Star (32px) | Actions (160px)

Between each column: a `col-sep` div (`width: 4px; cursor: col-resize`). Mousedown on sep → drag to resize adjacent columns. Column widths stored in component state (not persisted).

### 18.4 "Due in" Column

Formatted relative:
- Overdue: `Overdue` in red
- `< 1h`: `Xm` (e.g. `42m`)
- `1h–24h`: `Xh` (e.g. `3h`)
- `> 24h`: `Xd` (e.g. `2d`)

### 18.5 Actions Per Row

- **Open**: `navigateToOutline(task.id)`, close reminder window
- **✓ Complete**: `complete_task(id, true)`, remove from reminder list
- **✕ Dismiss**: add to `dismissed` list in localStorage, remove from view (permanent)
- **▾** (dropdown): snooze duration selector. Options: 5m 10m 15m 20m 30m 1h 2h 4h 8h 24h 2d 3d 4d 1w 2w
- **Snooze button**: add to `snoozed[taskId] = now + duration`, remove from view

Double-click row → same as Open.

---

## 19. Views Panel

Slide-in panel from right edge. Width: 260px. `position: absolute; right: 0; top: 0; bottom: 0`.

Toggle button in the titlebar (add a `⊟ views` icon button).

### 19.1 Layout

```
Views                              [+ New View]

[⠿] My Tasks                      [✎] [✕]
[⠿] This Week                     [✎] [✕]
[⠿] By Flag                       [✎] [✕]

──────────────────────────────────────────
When editing a view:

Name: [This Week          ]

Group by:  [Due Date     ▾]
Sort by:   [Due Date     ▾]
Order:     [Ascending    ▾]

☑ Show completed tasks

Visible columns:
☑ Flag  ☑ Tags  ☑ Due Date  ☐ Start Date
☑ Star  ☐ Note Preview  ☐ Reminder

[Cancel]  [Save]
```

Drag `⠿` handles to reorder views (updates tab order in titlebar too).

---

## 20. Keyboard Shortcuts

### Global

| Shortcut | Action |
|---|---|
| `Ctrl+Shift+I` | Open Rapid Input |
| `Ctrl+F` | Focus search in Outline toolbar |
| `Escape` | Close any open modal / clear selection |
| `F2` | Edit selected task caption |
| `Delete` | Delete selected task(s) |
| `Ctrl+Z` | Undo (last destructive action — delete or complete) |

### Outline / Task Tree

| Shortcut | Action |
|---|---|
| `Arrow Up/Down` | Move selection up/down |
| `Arrow Right` | Expand selected task |
| `Arrow Left` | Collapse selected task (or move to parent) |
| `Enter` | Enter inline edit on selected task |
| `Alt+Enter` | Enter inline edit and apply parsing on save |
| `Tab` (in edit) | Save, create sibling below, edit it |
| `Shift+Tab` (in edit) | Save, exit edit |
| `Ctrl+Enter` (in edit) | Add subtask |
| `Ctrl+D` | Duplicate task |
| `Ctrl+X` | Cut task |
| `Ctrl+C` | Copy task |
| `Ctrl+V` | Paste task (as sibling after selection) |
| `Space` | Toggle complete on selected task |
| `S` | Toggle star on selected task |
| `Ctrl+Arrow Up/Down` | Move task up/down within siblings |

---

## 21. Portable Data Storage

DB path resolution in `db.rs`:

```rust
pub fn db_path() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let data_dir = exe_dir.join("Data");
    std::fs::create_dir_all(&data_dir).ok();
    data_dir.join("tasks.db")
}
```

No AppData. No registry. The entire app is self-contained.

---

## 22. Database Encryption (Parked)

**Status: Do not implement until the Windows crash is resolved.**

The `bundled-sqlcipher-vendored-openssl` feature in rusqlite causes a pre-`main()` crash on Windows (OpenSSL static initializer fault). The app exits silently before any Rust code runs.

**When re-attempting:**
- Investigate using Windows CNG (Cryptography Next Generation) as the OpenSSL replacement for SQLCipher
- Or find a pre-built SQLCipher DLL for MSVC that avoids the static init issue
- Or ship OpenSSL DLLs alongside the exe

When implemented:
- On startup: try `db::open()` (no key). If file is encrypted, `open()` will fail → show LockScreen.
- LockScreen: password input, Unlock button. On submit: `open_with_key(password)`. Wrong password = error message, retry.
- Preferences → Security tab: Set / Change / Remove password via `PRAGMA rekey`.

---

## 23. Build & CI

### 23.1 GitHub Actions (`release.yml`)

- Trigger: `workflow_dispatch` or push of tag matching `v*`
- Runner: `windows-latest`
- Steps: checkout → Node 20 → Rust stable → Rust cache (`swatinem/rust-cache@v2`, `prefix-key: v2`) → `npm ci` → `npm run tauri build -- --bundles nsis` → upload two artifacts

### 23.2 Artifacts

| Artifact | Contents | Notes |
|---|---|---|
| `TaskClaw-portable` | `src-tauri/target/release/TaskClaw.exe` | Single exe, no install |
| `TaskClaw-installer` | `src-tauri/target/release/bundle/nsis/*.exe` | NSIS installer |

### 23.3 Critical Notes for CI

- **Always `git push origin main` before triggering `gh workflow run release.yml`**. The workflow runs whatever is on the remote, not the local working tree.
- Do not commit `src-tauri/Cargo.lock` — let CI resolve dependencies fresh on each build.
- `TAURI_SIGNING_PRIVATE_KEY` is set to empty string in CI (unsigned binary — users will see SmartScreen warning on first run).

---

## 24. Implementation Order

Build and test one feature at a time. Do not proceed to the next until the current one builds successfully, is pushed, and the user confirms it runs.

| Step | Feature | Rust changes | Frontend changes | Status |
|---|---|---|---|---|
| 1 | Theme & CSS variables | — | `app.css` | ✅ Done |
| 2 | Data model refactor | `types.rs`, `db.rs`, `commands/` | `api.ts`, `types.ts`, stores | ✅ Done |
| 3 | Portable DB path | `db.rs` | Prefs path display | ✅ Done |
| 4 | UI layout (tabs, detail panel, views panel) | — | All layout components | ✅ Done |
| 5 | TaskRow interactions (click model, hover actions, DnD) | — | `TaskRow.svelte` | ✅ Done |
| 6 | Right-click context menu | `commands/tasks.rs` (new commands) | `ContextMenu.svelte` | ✅ Done |
| 7 | Cross-view selection sync | stores | `TaskRow`, `GroupedView` | ✅ Done |
| 8 | Task Detail panel (full redesign) | — | `TaskDetail.svelte` + subsections | ✅ Done |
| 9 | Date + time fields | — | `TaskDetail.svelte` | ✅ Done |
| 10 | Reminders | — | `TaskDetail`, `ReminderWindow` | ✅ Done |
| 11 | Recurrence | `types.rs`, `db.rs`, `commands/` | `RecurrenceDialog.svelte` | ✅ Done |
| 12 | Rapid Input | — | `RapidInput.svelte` | ✅ Done |
| 13 | Alt+Enter inline parsing | — | `TaskRow.svelte` | ✅ Done |
| 14 | NLP date parser | — | `parsing.ts` | ✅ Done |
| 15 | GDrive OAuth sync + Web API + PlanView + ViewSettings + Prefs tabs | `commands/sync.rs`, `commands/webapi.rs`, `commands/files.rs` | `Prefs.svelte`, `PlanView.svelte`, `ViewSettingsDialog.svelte`, `SyncBar` | ✅ Done |
| 16 | Encryption | `Cargo.toml`, `db.rs`, `commands/` | `LockScreen`, Prefs | ⏸ Parked — see §22 |

### Parked features (not yet scheduled)

| Feature | Notes |
|---|---|
| Recurrence modal UI polish | Basic modal works; advanced options modal not yet built |
| Next Actions grouped display | Grouped view tab exists but Next Actions grouping not implemented |
| DB file permissions (Unix) | `0o600` chmod on DB creation — low priority, Windows primary target |

---

## 25. QA & Security Audit Log

### 25.1 QA Review — 2026-03-11 (commit `c7305e2`)

Reviewed against `94bd804`. GitHub issue: [#1](https://github.com/yellowcar1101/taskclaw/issues/1)

| ID | Severity | Finding | Remediation |
|---|---|---|---|
| QA-1 | Medium | Store subscription leaks in `TaskTree.svelte` | Fixed — `onDestroy` cleanup added |
| QA-2 | Medium | Store subscription leaks in `PlanView.svelte` | Fixed — `onDestroy` cleanup added |
| QA-3 | Medium | `RecurrenceDialog.svelte` reactive statement reset monthly mode incorrectly | Fixed — removed inverted reactive block |
| QA-4 | Low | Recurrence interval unbounded — extreme date offsets possible | Fixed — clamped to `.min(9999)` |
| QA-5 | Info | `contexts.rs` dead code (not in `mod.rs`) | No action — leave for cleanup |
| QA-6 | Info | 2 pre-existing TypeScript errors (pre-date this review) | Not introduced by this work |

### 25.2 Security Audit — 2026-03-11 (commit `c7305e2`)

Reviewed against `94bd804`. GitHub issue: [#2](https://github.com/yellowcar1101/taskclaw/issues/2)

| ID | Severity | Finding | Remediation |
|---|---|---|---|
| CRIT-1 | Critical | SQL injection in Web API `GET /tasks/:id` | Fixed — parameterized query |
| CRIT-2 | Critical | Web API startable with no token set | Fixed — token required before start |
| CRIT-3 | Critical | XSS in `TaskDetail` markdown renderer via `javascript:` URLs | Fixed — scheme allowlist (https?/mailto only) |
| HIGH-1 | High | `set_setting` accepted arbitrary keys (including credential keys) | Fixed — `ALLOWED_SETTING_KEYS` allowlist |
| HIGH-2 | High | Web API had no connection limit or read timeout | Fixed — max 50 connections, 5s read timeout |
| HIGH-3 | High | `lock().unwrap()` throughout all command files — mutex poison crashes process | Fixed — `map_err()?` or `match` pattern everywhere |
| HIGH-4 | High | No file extension validation on DB open/new/save paths | Fixed — `.db` extension enforced |
| HIGH-5 | High | No CSRF state in GDrive OAuth flow | Fixed — `uuid` state in `AuthInfo`, verified in `gdrive_wait_auth` |
| MED-3 | Medium | Recurrence interval unbounded (integer overflow) | Fixed — see QA-4 |
| MED-4 | Medium | No request body size limit in Web API | Fixed — 1MB cap |
| MED-5 | Medium | Web API CORS `Allow-Origin: *` | Fixed — restricted to `http://localhost` |
| MED-6 | Medium | `unwrap()` in `build_sync_payload` inner query | Fixed — `match`/`unwrap_or` |
| MED-7 | Medium | No CSP in `tauri.conf.json` | Fixed — restrictive CSP added |
| LOW-1 | Low | No Unix file permissions on DB file (0o600) | ⏸ Parked |
| LOW-2 | Low | `get_all_settings` returned credential keys to frontend | Fixed — credential keys excluded from query |
| LOW-5 | Low | No TCP read timeout on OAuth redirect listener | ⏸ Parked |
