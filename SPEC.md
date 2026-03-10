# TaskClaw — Feature Specification

> Working baseline: initial scaffold (task tree, contexts, tags, email links, dark theme, GDrive sync).
> This document captures all requested features to be added incrementally, in order of dependency.

---

## Current State (baseline)

- Hierarchical task tree with drag-and-drop reorder
- Inline task editing
- Contexts (coloured labels) and Tags
- Importance / Urgency / Effort scores → computed priority score
- Due date (date only, no time)
- Reminder field (datetime)
- Starred flag
- Email links per task
- Saved Views (filter + sort)
- GDrive sync (upload / download .db file)
- Dark theme
- DB stored in AppData

---

## Feature Roadmap

### 1 — Theme & Visual Polish

**Goal:** Lighter dark theme; grey-tinted inputs and surfaces so the UI feels less heavy.

- `--bg`: `#1a1a1a` (slightly lighter than pitch black)
- `--surface`: `#222222`, `--surface-elevated`: `#2a2a2a`
- `--border`: `#3c3c3c` (visible but not harsh)
- `--input-bg`: `#272727` (distinct from surface)
- `--text`: `#e0e0e0`, `--text-dim`: `#888888`
- Accent colour: `#4A9EFF` (blue)
- All inputs, selects, dropdowns use `--input-bg` background

---

### 2 — Data Model Refactor

**Replace Contexts with Flags; add start_date; add flag_id to Task.**

#### Why
MLO uses a single "flag" per task (a named colour badge, e.g. Urgent / Waiting). The current "contexts" concept is different. Keep Tags for multi-value labels.

#### Changes
- Remove `importance`, `urgency`, `effort`, `score` fields from Task (simplify — not used in practice)
- Remove `contexts` / `context_ids` from Task
- Add `flag_id TEXT` → FK to `flags` table (one flag per task, nullable)
- Add `start_date TEXT` (ISO date or datetime)
- `due_date` already exists — keep as-is
- `reminder_at` already exists — keep as-is

#### New `flags` table
```
id TEXT PK
name TEXT
color TEXT   -- hex
position REAL
```

#### Updated `saved_views` table
```
id, name, show_completed, group_by, sort_by, sort_dir, visible_fields[], position
```
- `group_by`: `none | flag | tag | due | start`
- `sort_by`: `position | due | start | caption | created`
- `sort_dir`: `asc | desc`
- `visible_fields`: array of field names to show in grouped view columns

#### Seed default flags
- 🔴 Urgent `#E05C5C`
- 🟡 Review `#D4A843`
- 🔵 Waiting `#4A9EFF`
- 🟢 Delegated `#6ABF69`

---

### 3 — UI Redesign

**Goal:** Tab-based layout in titlebar; task detail side panel; views panel.

#### Layout
```
┌─────────────────────────────────────────────────────┐
│ TaskClaw  [Outline] [View1] [View2]  …   ⚙  [Sync] │  ← titlebar
├──────────────────────────────────┬──────────────────┤
│                                  │                  │
│   Task tree / Grouped view       │  Task Detail     │
│                                  │  (shown when     │
│                                  │   task selected) │
│                                  │                  │
└──────────────────────────────────┴──────────────────┘
                                      ← Views Panel (slide-in from right)
```

#### Titlebar tabs
- "☰ Outline" always first
- Then one tab per Saved View
- Active tab highlighted with accent top border
- Preferences (⚙) icon button → opens Prefs modal
- SyncBar component (shows last sync time, upload/download buttons)

#### Outline tab
- Toolbar: `+ Task` | `📋 Rapid` | `[Search…]` | `⊞ expand` `⊟ collapse` `✕ clear`
- Column headers: Task | Start | Due
- TaskRow: toggle arrow | checkbox | flag dot | caption | start date | due date
- Hover actions: ✎ edit | ⭐ star | 🗑 delete | + add child
- Completed tasks hidden by default; shown when "show completed" is on in a View

#### Grouped View tab
- Tasks grouped by the view's `group_by` field
- Each group is a collapsible section with group header
- Columns controlled by `visible_fields`
- Double-click a task → navigate to Outline tab, scroll to that task with flash highlight
- Click a task → selects it (synced to Outline selection)

#### Task Detail panel (right side, shown when a task is selected)
- Caption (editable)
- Note (multi-line textarea)
- Flag (dropdown)
- Tags (multi-select chips)
- Start date + "Use time" checkbox + time input
- Due date + "Use time" checkbox + time input
- Reminder date + time
- Starred toggle
- Auto-fill: setting Start date → auto-fills Due date and Reminder if empty

#### Views Panel
- Slide-in panel (right edge) to manage saved views
- Add / rename / delete views
- Per-view settings: group_by, sort_by, sort_dir, show_completed, visible_fields

#### Preferences modal (tabs)
- **Flags tab**: list with colour swatch, add / edit / delete
- **Tags tab**: list with colour pill, add / delete

---

### 4 — Cross-View Selection Sync

- A single `selected` store (Set of task IDs)
- Selecting a task in any view (Outline or Grouped) updates the shared store
- Task Detail panel reacts to the store
- Double-click in Grouped View → `navigateToOutline(taskId)`:
  1. Set selected
  2. Expand all ancestors in the tree
  3. Switch active tab to "outline"
  4. Scroll to the row and flash-highlight it (CSS keyframe animation)

---

### 5 — Start / Due Date with Optional Time

- Both `start_date` and `due_date` store either `YYYY-MM-DD` or `YYYY-MM-DDTHH:mm`
- TaskDetail shows date input + "Use time" checkbox
- When "Use time" is checked → time input appears alongside
- When Start date is set and Due / Reminder are empty → auto-fill both to same value
- DateRow in TaskTree: show date portion only; colour-coded (overdue = red, today = amber, future = normal)

---

### 6 — Reminders

**Goal:** Per-task reminder with a floating non-modal reminder window.

#### TaskDetail
- Reminder: date input + time input (always has time)
- Defaults to start date/time when start is set
- Clear button to remove reminder

#### ReminderWindow component
- Floating panel, fixed position (top: 52px, centered)
- Shows tasks where `reminder_at <= now` AND not dismissed/snoozed
- Refreshed every 30 seconds
- **Columns** (resizable via drag on column separators): Task name | Flag | Due in | ⭐
- **Per-row actions**: Open Task | ✓ Complete | ✕ Dismiss (forever) | Snooze dropdown + button
- **Snooze options**: 5m 10m 15m 20m 30m 1h 2h 4h 8h 24h 2d 3d 4d 1w 2w
- State persisted in `localStorage` key `tc_reminders`: `{ dismissed: string[], snoozed: Record<string, ISO> }`
- Double-click row → navigate to task in Outline

---

### 7 — Rapid Input

**Goal:** MLO-style batch task entry from pasted indented text.

#### Trigger
- "📋 Rapid" button in Outline toolbar
- Keyboard: `Ctrl+Shift+I`

#### UI
- Modal dialog, full-width
- Left pane: textarea (paste indented text here)
- Right pane: live preview tree (shows parsed hierarchy + parsed fields as coloured badges)
- Top: parent task picker (searchable dropdown of full task tree with indentation)
- Checkbox: "Apply parsing" (on by default)
- Bottom syntax reference bar
- `Ctrl+Enter` → import | `Esc` → close

#### Indentation rules
- Each level of indentation (spaces or tabs) = one level deeper in hierarchy
- Consistent indentation detected automatically (2-space, 4-space, or tab)

#### Parsing (when "Apply parsing" is on)
- Same parsing engine as Alt+Enter (see §8)

---

### 8 — Inline Parsing (Alt+Enter)

**Goal:** When editing a task caption inline, `Alt+Enter` extracts structured fields from the text.

#### Trigger
- Any inline caption edit in TaskRow
- Press `Alt+Enter` instead of `Enter`

#### Parsed fields applied to task
- Caption (remainder after all tokens consumed)
- Flag: `!FlagName` or `-fl<name>`
- Tags: `#tagname` or `@tagname`
- Starred: `*` or `-star` or `-*`
- Start date: `s:` prefix or `-s` switch
- Due date: `d:` prefix or `-d` switch
- Reminder: `remind …` / `rmd …` suffix

---

### 9 — Natural Language Date Parsing

**Goal:** Full MLO-compatible date parsing engine.

#### Token formats (explicit)
- `s:2025-01-15` or `d:2025-01-15` — ISO date
- `s:+3d` — relative offset from today
- `s:today`, `s:tomorrow`
- `d:next friday`, `d:in 3 days`, `d:jan 26`

#### Inline NLP (no prefix — detected from sentence end)
- Right-to-left boundary scan: consume unambiguous date tokens from end of text
- "Call Jim tomorrow" → caption: "Call Jim", due: tomorrow
- "Buy 3 boxes" → no date (number not adjacent to unit)
- Quoted caption protects content: `"Buy 3 boxes"` → safe

#### Datetime expressions
- `in N unit` — relative: `in 3 days`, `in 2 weeks`, `in 1 month`
- Weekdays: `monday`, `next friday`
- Month+day: `jan 26`, `march 3rd`
- MM/DD: `3/15`
- Time: `3pm`, `15:10`, `at 3pm`, `3 pm`
- Combined: `in 3 days 4pm`, `next friday 9am`, `tomorrow at 2:30`

#### Reminder expressions (after `remind` / `rmd` keyword)
- `remind 10 min in advance` → due_time − 10 min
- `remind 1 hour before` → due_time − 1 hour
- `remind me tomorrow 9am` → absolute datetime
- `remind me` → same as due date/time

#### Switch tokens
- `-fl<name>` → set flag by name (fuzzy match)
- `-star` / `-*` → starred = true
- `-s` → next date expression is start date
- `-d` → next date expression is due date
- `*` standalone → starred

---

### 10 — Portable Data Storage

**Goal:** DB lives next to the exe, not in AppData. Fully portable.

- DB path: `<exe directory>/Data/tasks.db`
- `Data/` directory created automatically on first run
- Move or delete the whole folder → no traces left anywhere

---

### 11 — Database Encryption (optional, user-controlled)

**Goal:** Password-protect the DB with AES-256 encryption.

#### Implementation
- SQLCipher (`rusqlite` with `bundled-sqlcipher-vendored-openssl`)
- On startup: try open without key → if fails → show LockScreen
- LockScreen: password input, Unlock button, error on wrong password
- On unlock: `PRAGMA key='...'` → verify → store open connection

#### Commands
- `is_db_locked()` → bool
- `unlock_db(password)` → Result
- `set_db_password(new_password)` → Result (empty string = remove encryption, `PRAGMA rekey`)

#### Preferences → Security tab
- Status badge: 🔒 Encrypted / 🔓 Not encrypted
- Set Password form (new + confirm)
- Change Password form (new + confirm)
- Remove Password button
- DB file path display

#### Known issue
- `bundled-sqlcipher-vendored-openssl` crashes on Windows before `main()` runs
  (OpenSSL static initializer). Needs investigation before re-enabling.
  Options: use Windows CNG instead of OpenSSL, or find a pre-built SQLCipher for MSVC.

---

## Build & Delivery

- CI: GitHub Actions, `windows-latest`, Rust stable + Node 20
- Artifacts: `TaskClaw-portable` (bare `.exe`) + `TaskClaw-installer` (NSIS)
- DB: `<exe folder>/Data/tasks.db` — portable, no install required
- **Always `git push` before triggering `gh workflow run`**

---

## Implementation Order

| # | Feature | Rust changes | Frontend changes |
|---|---------|-------------|-----------------|
| 1 | Theme | — | app.css |
| 2 | Data model refactor | types.rs, db.rs, commands/ | api.ts, types.ts, stores |
| 3 | UI redesign | — | all components |
| 4 | Cross-view selection | stores (outlineScrollToId) | TaskRow, GroupedView |
| 5 | Date + time fields | — | TaskDetail |
| 6 | Reminders | — | TaskDetail, ReminderWindow |
| 7 | Rapid Input | — | RapidInput.svelte |
| 8 | Alt+Enter parsing | — | TaskRow |
| 9 | NLP date parser | — | parsing.ts |
| 10 | Portable DB path | db.rs | Prefs (path display) |
| 11 | Encryption | Cargo.toml, db.rs, commands/ | LockScreen, Prefs |
