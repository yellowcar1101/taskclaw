<script lang="ts">
  import { writable } from 'svelte/store';
  import type { SavedView, Task } from '../types';
  import {
    allTasks, searchQuery, filterFlagId, sortTasks,
    contextMenu, setSelected, detailTaskId, updateTask, completeTask
  } from '../stores/tasks';
  import { filterTasksForView, groupTasks } from '../planFilter';
  import type { ActionFilter, TaskGroup } from '../planFilter';
  import { formatDateDisplay, dateClass } from '../parsing';

  export let view: SavedView;

  let collapsedGroups = new Set<string>();

  $: filterObj = (() => {
    try { return JSON.parse(view.filter_json || '{}'); } catch { return {}; }
  })();
  $: actionFilter = (filterObj.action_filter ?? 'all') as ActionFilter;

  $: filtered = filterTasksForView($allTasks, {
    actionFilter,
    showCompleted: view.show_completed,
    searchQuery: $searchQuery,
    flagId: $filterFlagId,
  });

  $: sorted = sortTasks(filtered, (view.sort_by || 'position') as any, (view.sort_dir || 'asc') as any);
  $: groups = groupTasks(sorted, view.group_by || 'none');

  function toggleGroup(key: string) {
    collapsedGroups = new Set(collapsedGroups);
    if (collapsedGroups.has(key)) collapsedGroups.delete(key);
    else collapsedGroups.add(key);
  }

  function onRowClick(e: MouseEvent, task: Task) {
    setSelected(task.id, e.ctrlKey || e.metaKey);
  }

  function onContextMenu(e: MouseEvent, taskId: string) {
    e.preventDefault();
    contextMenu.set({ x: e.clientX, y: e.clientY, taskId });
  }

  async function onComplete(task: Task) {
    await completeTask(task.id, true);
  }

  function formatDue(due: string | null): string {
    if (!due) return '';
    const d = new Date(due + (due.length === 10 ? 'T00:00:00' : ''));
    const diff = Math.floor((d.getTime() - Date.now()) / 86400000);
    if (diff < 0) return `${Math.abs(diff)}d overdue`;
    if (diff === 0) return 'Today';
    if (diff === 1) return 'Tomorrow';
    if (diff <= 7) return `${diff}d`;
    return d.toLocaleDateString('en-GB', { day: 'numeric', month: 'short' });
  }

  function isStartOverdue(startDate: string | null): boolean {
    if (!startDate) return false;
    if (startDate.includes('T')) return new Date(startDate).getTime() < Date.now();
    return startDate < new Date().toISOString().slice(0, 10);
  }

  // ── Column visibility ──────────────────────────────────────────────────────
  const COL_KEY = 'plan_visible_cols';
  function loadCols(): Set<string> {
    try { return new Set(JSON.parse(localStorage.getItem(COL_KEY) ?? '["due","start"]')); }
    catch { return new Set(['due', 'start']); }
  }
  let visibleCols = writable<Set<string>>(loadCols());
  visibleCols.subscribe(v => localStorage.setItem(COL_KEY, JSON.stringify([...v])));

  const ALL_COLS = [
    { id: 'due',   label: 'Due Date' },
    { id: 'start', label: 'Start Date' },
    { id: 'flag',  label: 'Flag' },
    { id: 'tags',  label: 'Tags' },
  ];
  let colMenu: { x: number; y: number } | null = null;

  function onHeaderContextMenu(e: MouseEvent) {
    e.preventDefault();
    colMenu = { x: e.clientX, y: e.clientY };
  }

  function toggleCol(id: string) {
    visibleCols.update(s => {
      const n = new Set(s);
      n.has(id) ? n.delete(id) : n.add(id);
      return n;
    });
  }
</script>

<div class="plan-view">
  <!-- Column headers — right-click to toggle columns -->
  <div class="col-headers" on:contextmenu={onHeaderContextMenu} role="none">
    <div class="col-spacer"></div>
    <div class="col-header caption-col">Task</div>
    {#if $visibleCols.has('due')}
      <div class="col-header due-col">Due</div>
    {/if}
    {#if $visibleCols.has('start')}
      <div class="col-header start-col">Start</div>
    {/if}
    {#if $visibleCols.has('flag')}
      <div class="col-header flag-col">Flag</div>
    {/if}
    {#if $visibleCols.has('tags')}
      <div class="col-header tags-col">Tags</div>
    {/if}
    <div class="col-hint" title="Right-click to show/hide columns">⋮</div>
  </div>

  <!-- Right-click column menu -->
  {#if colMenu}
    <div class="col-menu-backdrop" on:click={() => colMenu = null} role="none"></div>
    <div class="col-menu-popup" style="left:{colMenu.x}px;top:{colMenu.y}px" role="menu">
      <div class="col-menu-title">Columns</div>
      {#each ALL_COLS as col}
        <label class="col-menu-item">
          <input type="checkbox" checked={$visibleCols.has(col.id)} on:change={() => toggleCol(col.id)} />
          {col.label}
        </label>
      {/each}
    </div>
  {/if}

  <!-- Header stats -->
  <div class="plan-header">
    <span class="plan-count">{sorted.length} task{sorted.length === 1 ? '' : 's'}</span>
    {#if actionFilter !== 'all'}
      <span class="filter-badge">{actionFilter.replace('_', ' ')}</span>
    {/if}
  </div>

  <div class="plan-list">
    {#each groups as group (group.key)}
      <!-- Group header (only when grouped) -->
      {#if group.label}
        <button
          class="group-header"
          on:click={() => toggleGroup(group.key)}
        >
          <span class="group-arrow">{collapsedGroups.has(group.key) ? '▶' : '▼'}</span>
          {#if group.color}
            <span class="group-dot" style="background:{group.color}"></span>
          {/if}
          <span class="group-label">{group.label}</span>
          <span class="group-count">{group.tasks.length}</span>
        </button>
      {/if}

      <!-- Group tasks -->
      {#if !collapsedGroups.has(group.key)}
        {#each group.tasks as task (task.id)}
          <div
            class="plan-row"
            on:click={e => onRowClick(e, task)}
            on:contextmenu={e => onContextMenu(e, task.id)}
            role="none"
            tabindex="-1"
          >
            <!-- Complete checkbox -->
            <button
              class="check-box"
              class:start-overdue={isStartOverdue(task.start_date)}
              on:click|stopPropagation={() => onComplete(task)}
              title="Mark complete"
              tabindex="-1"
            ></button>

            <!-- Caption area -->
            <div class="row-caption">
              <span class="caption-text" class:starred={task.starred}>
                {task.caption}
              </span>
              {#if task.flag && !$visibleCols.has('flag')}
                <span class="flag-dot" style="background:{task.flag.color}" title={task.flag.name}></span>
              {/if}
              {#if !$visibleCols.has('tags')}
                {#each task.tags as tag}
                  <span class="tag-chip" style="background:{tag.color}22;color:{tag.color};border-color:{tag.color}44">
                    {tag.name}
                  </span>
                {/each}
              {/if}
            </div>

            <!-- Due date column -->
            {#if $visibleCols.has('due')}
              <div class="col-due {dateClass(task.due_date)}">{formatDue(task.due_date)}</div>
            {/if}

            <!-- Start date column -->
            {#if $visibleCols.has('start')}
              <div class="col-start">{task.start_date ? task.start_date.split('T')[0] : ''}</div>
            {/if}

            <!-- Flag column — dot only -->
            {#if $visibleCols.has('flag')}
              <div class="col-flag">
                {#if task.flag}
                  <span class="col-dot" style="background:{task.flag.color}" title={task.flag.name}></span>
                {/if}
              </div>
            {/if}

            <!-- Tags column — dots only -->
            {#if $visibleCols.has('tags')}
              <div class="col-tags">
                {#each task.tags.slice(0, 4) as tag}
                  <span class="col-dot" style="background:{tag.color}" title={tag.name}></span>
                {/each}
              </div>
            {/if}
          </div>
        {/each}
      {/if}
    {/each}

    {#if sorted.length === 0}
      <div class="empty">
        {#if actionFilter === 'active'}
          No active tasks — everything is done or future-dated.
        {:else if actionFilter === 'next_actions'}
          No next actions found.
        {:else}
          No tasks match the current filter.
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .plan-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  /* ── Column header bar ── */
  .col-headers {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 6px;
    height: 22px;
    background: var(--surface-elevated);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    user-select: none;
    cursor: default;
  }
  .col-spacer { width: 28px; flex-shrink: 0; }
  .col-header {
    color: var(--text-dim);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 0 2px;
  }
  .col-header.caption-col { flex: 1; }
  .col-header.due-col     { width: 80px; flex-shrink: 0; text-align: right; }
  .col-header.start-col   { width: 70px; flex-shrink: 0; text-align: right; }
  .col-header.flag-col    { width: 30px; flex-shrink: 0; text-align: center; }
  .col-header.tags-col    { width: 52px; flex-shrink: 0; text-align: center; }
  .col-hint {
    color: var(--text-dim);
    font-size: 13px;
    opacity: 0.4;
    flex-shrink: 0;
    padding: 0 2px;
  }
  .col-headers:hover .col-hint { opacity: 0.8; }

  /* ── Column right-click menu ── */
  .col-menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: 498;
  }
  .col-menu-popup {
    position: fixed;
    z-index: 499;
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.4);
    padding: 4px 0;
    min-width: 150px;
  }
  .col-menu-title {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-dim);
    padding: 4px 12px 2px;
  }
  .col-menu-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 12px;
    font-size: 12px;
    color: var(--text);
    cursor: pointer;
  }
  .col-menu-item:hover { background: var(--hover); }

  /* ── Stats bar ── */
  .plan-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 10px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    background: var(--surface-elevated);
    font-size: 11px;
  }
  .plan-count { color: var(--text-dim); }
  .filter-badge {
    background: var(--accent-dim);
    color: var(--accent);
    padding: 1px 6px;
    border-radius: 8px;
    font-size: 10px;
    text-transform: capitalize;
  }

  .plan-list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }

  .group-header {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    background: none;
    border: none;
    border-bottom: 1px solid var(--border);
    color: var(--text-dim);
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 6px 12px;
    cursor: pointer;
    text-align: left;
    margin-top: 4px;
  }
  .group-header:hover { background: var(--hover); color: var(--text); }
  .group-arrow { font-size: 9px; width: 12px; }
  .group-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
  .group-label { flex: 1; }
  .group-count {
    background: var(--hover-btn);
    border-radius: 8px;
    padding: 0 5px;
    font-size: 10px;
    font-weight: normal;
    color: var(--text-dim);
  }

  /* ── Task rows ── */
  .plan-row {
    display: flex;
    align-items: center;
    gap: 4px;
    height: var(--row-height, 28px);
    padding: 0 8px 0 8px;
    cursor: default;
    border-radius: 3px;
    margin: 0 4px;
    color: var(--text);
    user-select: none;
  }
  .plan-row:hover { background: var(--hover); }

  .check-box {
    width: 15px;
    height: 15px;
    border: 1.5px solid var(--text-dim);
    border-radius: 3px;
    background: transparent;
    cursor: pointer;
    flex-shrink: 0;
    padding: 0;
    transition: border-color 0.1s, background 0.1s;
  }
  .check-box:hover { border-color: var(--green); background: rgba(106,191,105,0.15); }
  .check-box.start-overdue { border-color: var(--red); background: rgba(255,255,255,0.06); }
  .check-box.start-overdue:hover { border-color: var(--red); background: rgba(224,92,92,0.15); }

  .row-caption {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 5px;
    overflow: hidden;
    min-width: 0;
  }

  .caption-text {
    font-family: var(--app-font);
    font-size: var(--app-font-size, 13px);
    color: var(--task-color, var(--text));
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    min-width: 0;
  }
  .caption-text.starred::before { content: '★ '; color: var(--amber); }

  .flag-dot { width: 7px; height: 7px; border-radius: 50%; flex-shrink: 0; }
  .tag-chip {
    font-size: 10px;
    padding: 1px 4px;
    border-radius: 8px;
    border: 1px solid;
    white-space: nowrap;
    flex-shrink: 0;
  }

  /* ── Column cells ── */
  .col-due {
    font-size: 11px;
    color: var(--text-dim);
    width: 80px;
    text-align: right;
    flex-shrink: 0;
  }
  .col-due.overdue { color: var(--red); font-weight: 600; }
  .col-due.today   { color: var(--amber); font-weight: 600; }
  .col-due.soon    { color: var(--amber); }

  .col-start {
    font-size: 11px;
    color: var(--text-dim);
    width: 70px;
    text-align: right;
    flex-shrink: 0;
  }

  .col-flag {
    width: 30px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .col-tags {
    width: 52px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 3px;
    justify-content: center;
  }
  .col-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
    display: inline-block;
  }

  .empty {
    padding: 40px 20px;
    color: var(--text-dim);
    font-size: 12px;
    text-align: center;
  }
</style>
