<script lang="ts">
  import { allTasks, completeTask, navigateToOutline, openDetail } from '../stores/tasks';
  import type { Task } from '../types';

  const STORAGE_KEY = 'tc_reminders';
  const SNOOZE_OPTIONS = ['5m','10m','15m','20m','30m','1h','2h','4h','8h','24h','2d','3d','4d','1w','2w'];

  function snoozeMs(opt: string): number {
    const n = parseInt(opt);
    if (opt.endsWith('m'))  return n * 60 * 1000;
    if (opt.endsWith('h'))  return n * 3600 * 1000;
    if (opt.endsWith('d'))  return n * 86400 * 1000;
    if (opt.endsWith('w'))  return n * 7 * 86400 * 1000;
    return 0;
  }

  interface ReminderState { dismissed: string[]; snoozed: Record<string, string>; }

  function loadState(): ReminderState {
    try { return JSON.parse(localStorage.getItem(STORAGE_KEY) || '{}'); } catch { return {}; }
  }
  function saveState(s: ReminderState) {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(s));
  }

  let active: Task[] = [];
  let snoozeSelections: Record<string, string> = {};
  let visible = true;

  export function refresh() {
    const state = loadState();
    const dismissed = new Set(state.dismissed ?? []);
    const snoozed = state.snoozed ?? {};
    const now = new Date();
    active = $allTasks.filter(task => {
      if (!task.reminder_at || task.completed_at) return false;
      if (dismissed.has(task.id)) return false;
      if (snoozed[task.id] && new Date(snoozed[task.id]) > now) return false;
      return new Date(task.reminder_at) <= now;
    });
    if (active.length > 0) visible = true;
    // init snooze selections
    active.forEach(t => { if (!snoozeSelections[t.id]) snoozeSelections[t.id] = '15m'; });
  }

  function dismiss(taskId: string) {
    const s = loadState();
    s.dismissed = [...(s.dismissed ?? []), taskId];
    saveState(s);
    active = active.filter(t => t.id !== taskId);
  }

  function snooze(taskId: string) {
    const duration = snoozeSelections[taskId] ?? '15m';
    const until = new Date(Date.now() + snoozeMs(duration)).toISOString();
    const s = loadState();
    s.snoozed = { ...(s.snoozed ?? {}), [taskId]: until };
    saveState(s);
    active = active.filter(t => t.id !== taskId);
  }

  async function complete(taskId: string) {
    await completeTask(taskId, true);
    active = active.filter(t => t.id !== taskId);
  }

  function openTask(taskId: string) {
    navigateToOutline(taskId);
    openDetail(taskId);
  }

  function formatDueIn(d: string | null): string {
    if (!d) return '—';
    const diff = Math.floor((new Date(d.includes('T') ? d : d + 'T00:00:00').getTime() - Date.now()) / 60000);
    if (diff < 0)    return `${Math.abs(diff) < 60 ? `${Math.abs(diff)}m` : Math.abs(diff) < 1440 ? `${Math.floor(Math.abs(diff)/60)}h` : `${Math.floor(Math.abs(diff)/1440)}d`} ago`;
    if (diff === 0)  return 'Now';
    if (diff < 60)   return `${diff}m`;
    if (diff < 1440) return `${Math.floor(diff/60)}h`;
    return `${Math.floor(diff/1440)}d`;
  }

  // Column widths (px), resizable via drag
  let colWidths = { name: 200, flag: 56, dueIn: 76, star: 36 };
  let resizing: keyof typeof colWidths | null = null;
  let resizeStartX = 0;
  let resizeStartW = 0;

  function onResizeStart(e: MouseEvent, col: keyof typeof colWidths) {
    e.preventDefault();
    resizing = col;
    resizeStartX = e.clientX;
    resizeStartW = colWidths[col];
  }

  function onMouseMove(e: MouseEvent) {
    if (!resizing) return;
    const delta = e.clientX - resizeStartX;
    colWidths[resizing] = Math.max(30, resizeStartW + delta);
    colWidths = { ...colWidths };
  }

  function onMouseUp() { resizing = null; }
</script>

<svelte:window on:mousemove={onMouseMove} on:mouseup={onMouseUp} />

{#if active.length > 0 && visible}
  <div class="reminder-window">
    <div class="rw-header">
      <span class="rw-title">Reminders ({active.length})</span>
      <button class="rw-close" on:click={() => visible = false} title="Hide">—</button>
    </div>

    <!-- Column headers -->
    <div class="rw-thead">
      <div class="rw-th" style="width:{colWidths.name}px">
        Task name
        <div class="col-sep" role="separator" on:mousedown={e => onResizeStart(e, 'name')}></div>
      </div>
      <div class="rw-th" style="width:{colWidths.flag}px">
        Flag
        <div class="col-sep" role="separator" on:mousedown={e => onResizeStart(e, 'flag')}></div>
      </div>
      <div class="rw-th" style="width:{colWidths.dueIn}px">
        Due in
        <div class="col-sep" role="separator" on:mousedown={e => onResizeStart(e, 'dueIn')}></div>
      </div>
      <div class="rw-th" style="width:{colWidths.star}px">
        ★
        <div class="col-sep" role="separator" on:mousedown={e => onResizeStart(e, 'star')}></div>
      </div>
      <div class="rw-th actions-th">Actions</div>
    </div>

    <!-- Rows -->
    <div class="rw-body">
      {#each active as task (task.id)}
        <div class="rw-row" on:dblclick={() => openTask(task.id)} role="row">
          <!-- Name -->
          <div class="rw-cell name-cell" style="width:{colWidths.name}px" title={task.caption}>
            {#if task.flag}
              <span class="flag-mini" style="background:{task.flag.color}"></span>
            {:else}
              <span class="flag-mini empty"></span>
            {/if}
            <span class="task-name">{task.caption}</span>
          </div>

          <!-- Flag label -->
          <div class="rw-cell" style="width:{colWidths.flag}px">
            {#if task.flag}
              <span class="flag-pill" style="background:{task.flag.color}22;color:{task.flag.color};border-color:{task.flag.color}55">
                {task.flag.name}
              </span>
            {/if}
          </div>

          <!-- Due in -->
          <div class="rw-cell due-cell" style="width:{colWidths.dueIn}px"
            class:overdue={task.due_date && new Date(task.due_date.includes('T') ? task.due_date : task.due_date + 'T00:00:00') < new Date()}>
            {formatDueIn(task.due_date)}
          </div>

          <!-- Star -->
          <div class="rw-cell star-cell" style="width:{colWidths.star}px">
            {#if task.starred}<span class="star-icon">★</span>{/if}
          </div>

          <!-- Actions -->
          <div class="rw-cell actions-cell">
            <button class="ra-btn" on:click={() => openTask(task.id)} title="Open Task">Open</button>
            <button class="ra-btn complete" on:click={() => complete(task.id)} title="Complete">✓</button>
            <button class="ra-btn dismiss" on:click={() => dismiss(task.id)} title="Dismiss forever">✕</button>
            <select class="snooze-sel" bind:value={snoozeSelections[task.id]}>
              {#each SNOOZE_OPTIONS as opt}
                <option value={opt}>{opt}</option>
              {/each}
            </select>
            <button class="ra-btn snooze" on:click={() => snooze(task.id)}>Snooze</button>
          </div>
        </div>
      {/each}
    </div>

    <div class="rw-footer">
      <span class="rw-hint">Double-click a task to go to it in the Outline</span>
    </div>
  </div>
{/if}

<style>
  .reminder-window {
    position: fixed;
    top: 52px; left: 50%;
    transform: translateX(-50%);
    min-width: 640px; max-width: 92vw;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 8px 32px rgba(0,0,0,0.55);
    z-index: 500;
    display: flex; flex-direction: column;
    overflow: hidden;
    max-height: 60vh;
  }

  .rw-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 8px 12px; background: var(--surface-elevated);
    border-bottom: 1px solid var(--border); flex-shrink: 0;
  }
  .rw-title { font-size: 12px; font-weight: 600; color: var(--accent); font-family: sans-serif; letter-spacing: 0.04em; }
  .rw-close { background: none; border: none; color: var(--text-dim); cursor: pointer; font-size: 14px; padding: 1px 6px; border-radius: 3px; }
  .rw-close:hover { color: var(--text); background: var(--hover); }

  /* Column headers */
  .rw-thead {
    display: flex; align-items: stretch;
    background: var(--surface-elevated);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0; overflow: hidden;
  }
  .rw-th {
    font-size: 10px; color: var(--text-dim); font-family: sans-serif;
    text-transform: uppercase; letter-spacing: 0.05em;
    padding: 4px 8px; flex-shrink: 0; position: relative;
    overflow: hidden; white-space: nowrap;
    display: flex; align-items: center;
  }
  .actions-th { flex: 1; }

  .col-sep {
    position: absolute; right: 0; top: 0; bottom: 0;
    width: 4px; cursor: col-resize;
    background: transparent;
  }
  .col-sep:hover { background: var(--accent); }

  /* Body */
  .rw-body { overflow-y: auto; flex: 1; }

  .rw-row {
    display: flex; align-items: center;
    border-bottom: 1px solid var(--border);
    min-height: 40px; cursor: default;
    transition: background 0.06s;
  }
  .rw-row:hover { background: var(--hover); }
  .rw-row:last-child { border-bottom: none; }

  .rw-cell {
    padding: 4px 8px; flex-shrink: 0; font-size: 12px;
    overflow: hidden; display: flex; align-items: center; gap: 4px;
  }

  .name-cell { gap: 6px; }
  .task-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-family: 'Cascadia Code', 'Fira Code', monospace; font-size: 12px; color: var(--text); }

  .flag-mini { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
  .flag-mini.empty { background: transparent; border: 1px solid var(--border); }

  .flag-pill { font-size: 10px; padding: 1px 6px; border-radius: 8px; border: 1px solid; white-space: nowrap; font-family: sans-serif; }

  .due-cell { font-family: sans-serif; color: var(--text-dim); font-size: 11px; }
  .due-cell.overdue { color: var(--red); font-weight: 600; }

  .star-cell { justify-content: center; }
  .star-icon { color: var(--gold); font-size: 13px; }

  .actions-cell { flex: 1; gap: 4px; flex-wrap: nowrap; }

  .ra-btn {
    background: var(--hover-btn); border: 1px solid var(--border);
    color: var(--text-dim); padding: 3px 8px; border-radius: 3px;
    cursor: pointer; font-size: 11px; white-space: nowrap; font-family: sans-serif;
    transition: background 0.1s, color 0.1s;
    flex-shrink: 0;
  }
  .ra-btn:hover { color: var(--text); background: var(--hover); }
  .ra-btn.complete:hover { color: var(--green); }
  .ra-btn.dismiss:hover  { color: var(--red); }
  .ra-btn.snooze { background: var(--accent); color: #fff; border-color: var(--accent); }
  .ra-btn.snooze:hover { filter: brightness(1.15); }

  .snooze-sel {
    background: var(--input-bg); border: 1px solid var(--border);
    color: var(--text); padding: 2px 4px; border-radius: 3px;
    font-size: 11px; outline: none; flex-shrink: 0;
    font-family: sans-serif;
  }

  .rw-footer {
    padding: 4px 12px; border-top: 1px solid var(--border);
    flex-shrink: 0;
  }
  .rw-hint { font-size: 10px; color: var(--text-dim); font-family: sans-serif; }
</style>
