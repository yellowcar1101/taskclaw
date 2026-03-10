<script lang="ts">
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
</script>

<div class="plan-view">
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
            class:selected={false}
            on:click={e => onRowClick(e, task)}
            on:contextmenu={e => onContextMenu(e, task.id)}
            role="row"
            tabindex="0"
            on:keydown
          >
            <!-- Complete button -->
            <button class="check-btn" on:click|stopPropagation={() => onComplete(task)} title="Complete">○</button>

            <!-- Caption area -->
            <div class="row-caption">
              <span class="caption-text" class:starred={task.starred}>
                {task.caption}
              </span>
              {#if task.flag}
                <span class="flag-dot" style="background:{task.flag.color}" title={task.flag.name}></span>
              {/if}
              {#each task.tags as tag}
                <span class="tag-chip" style="background:{tag.color}22;color:{tag.color};border-color:{tag.color}44">
                  {tag.name}
                </span>
              {/each}
            </div>

            <!-- Start date -->
            {#if task.start_date}
              <span class="date-cell dim">{formatDateDisplay(task.start_date)}</span>
            {/if}

            <!-- Due date -->
            <span class="date-cell {dateClass(task.due_date)}">
              {formatDue(task.due_date)}
            </span>
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

  .plan-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
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

  .plan-row {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 8px 0 24px;
    cursor: default;
    border-radius: 3px;
    margin: 0 4px;
    color: var(--text);
    user-select: none;
  }
  .plan-row:hover { background: var(--hover); }
  .plan-row.selected { background: var(--selected); }

  .check-btn {
    background: none;
    border: none;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 12px;
    padding: 2px;
    flex-shrink: 0;
  }
  .check-btn:hover { color: var(--green); }

  .row-caption {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 5px;
    overflow: hidden;
    min-width: 0;
  }

  .caption-text {
    font-size: 12px;
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

  .date-cell {
    font-size: 11px;
    color: var(--text-dim);
    flex-shrink: 0;
    min-width: 60px;
    text-align: right;
  }
  .date-cell.overdue { color: var(--red); font-weight: 600; }
  .date-cell.today   { color: var(--amber); font-weight: 600; }
  .date-cell.dim     { color: var(--text-dim); opacity: 0.6; }

  .empty {
    padding: 40px 20px;
    color: var(--text-dim);
    font-size: 12px;
    text-align: center;
  }
</style>
