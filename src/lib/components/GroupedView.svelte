<script lang="ts">
  import type { Task, SavedView, TaskGroup } from '../types';
  import {
    allTasks, flags, sortTasks, groupTasks,
    toggleGroup, collapsedGroups, selected, setSelected,
    editingId, updateTask, deleteTask, completeTask, openDetail, createTask
  } from '../stores/tasks';
  import { api } from '../api';

  export let view: SavedView;

  let editingTaskId: string | null = null;
  let editValue = '';
  let inputEl: HTMLInputElement;

  $: activeTasks = $allTasks.filter(t => view.show_completed ? true : !t.completed_at);
  $: sorted = sortTasks(activeTasks, view.sort_by as any, view.sort_dir as any);
  $: groups = groupTasks(sorted, view.group_by as any, $flags);

  function formatDate(d: string | null): string {
    if (!d) return '';
    const dt = new Date(d + 'T00:00:00');
    const today = new Date(); today.setHours(0,0,0,0);
    const diff = Math.floor((dt.getTime() - today.getTime()) / 86400000);
    if (diff < 0)  return `${Math.abs(diff)}d ago`;
    if (diff === 0) return 'Today';
    if (diff === 1) return 'Tomorrow';
    if (diff <= 7)  return `${diff}d`;
    return dt.toLocaleDateString('en-GB', { day: 'numeric', month: 'short' });
  }

  function dateClass(d: string | null): string {
    if (!d) return '';
    const diff = Math.floor((new Date(d + 'T00:00:00').getTime() - Date.now()) / 86400000);
    if (diff < 0) return 'overdue'; if (diff === 0) return 'today'; if (diff <= 3) return 'soon'; return '';
  }

  function startInlineEdit(task: Task) {
    editingTaskId = task.id;
    editValue = task.caption;
    setTimeout(() => inputEl?.focus(), 10);
  }

  async function commitEdit(task: Task) {
    if (editValue.trim() && editValue !== task.caption)
      await updateTask(task.id, { caption: editValue.trim() });
    editingTaskId = null;
  }

  function onKeydown(e: KeyboardEvent, task: Task) {
    if (e.key === 'Enter') { e.preventDefault(); commitEdit(task); }
    if (e.key === 'Escape') editingTaskId = null;
  }
</script>

<div class="grouped-view">
  {#each groups as group (group.key)}
    {@const collapsed = $collapsedGroups.has(group.key)}

    {#if group.label}
      <div class="group-header" on:click={() => toggleGroup(group.key)} role="button" tabindex="0"
        on:keydown={e => e.key === 'Enter' && toggleGroup(group.key)}>
        <span class="group-chevron">{collapsed ? '▸' : '▾'}</span>
        {#if group.color}
          <span class="group-dot" style="background:{group.color}"></span>
        {/if}
        <span class="group-label">{group.label}</span>
        <span class="group-count">{group.tasks.length}</span>
      </div>
    {/if}

    {#if !collapsed}
      {#each group.tasks as task (task.id)}
        <div
          class="view-row"
          class:selected={$selected.has(task.id)}
          class:completed={!!task.completed_at}
          on:click={e => setSelected(task.id, e.ctrlKey || e.metaKey)}
          on:dblclick={() => startInlineEdit(task)}
          role="row"
          tabindex="0"
        >
          <!-- Complete -->
          <button class="icon-btn check" on:click|stopPropagation={() => completeTask(task.id, !task.completed_at)}
            tabindex="-1">{task.completed_at ? '●' : '○'}</button>

          <!-- Flag dot -->
          {#if task.flag}
            <span class="flag-dot" style="background:{task.flag.color}" title={task.flag.name}></span>
          {:else}
            <span class="flag-dot empty"></span>
          {/if}

          <!-- Caption -->
          <div class="caption-cell">
            {#if editingTaskId === task.id}
              <input bind:this={inputEl} bind:value={editValue} class="caption-input"
                on:blur={() => commitEdit(task)} on:keydown={e => onKeydown(e, task)} />
            {:else}
              <span class="caption-text" class:completed-text={!!task.completed_at}>{task.caption}</span>
            {/if}
            {#each task.tags as tag}
              <span class="tag-chip" style="background:{tag.color}22;color:{tag.color};border-color:{tag.color}55">{tag.name}</span>
            {/each}
            {#if task.note}<span class="note-dot">¶</span>{/if}
          </div>

          <!-- Start date -->
          <div class="col-date {dateClass(task.start_date)}">{formatDate(task.start_date)}</div>
          <!-- Due date -->
          <div class="col-date {dateClass(task.due_date)}">{formatDate(task.due_date)}</div>

          <!-- Actions on hover -->
          <div class="row-actions">
            <button class="icon-btn" on:click|stopPropagation={() => openDetail(task.id)} title="Detail">⋯</button>
            <button class="icon-btn danger" on:click|stopPropagation={async () => {
              if (confirm(`Delete "${task.caption}"?`)) await deleteTask(task.id);
            }} title="Delete">✕</button>
          </div>
        </div>
      {/each}
    {/if}
  {/each}
</div>

<style>
  .grouped-view { flex: 1; overflow-y: auto; padding: 4px 2px; }

  .group-header {
    display: flex; align-items: center; gap: 6px;
    padding: 6px 8px 4px;
    cursor: pointer; user-select: none;
    color: var(--text-dim); font-size: 11px;
    font-family: sans-serif; text-transform: uppercase; letter-spacing: 0.05em;
    border-bottom: 1px solid var(--border); margin-bottom: 2px;
  }
  .group-header:hover { color: var(--text); }
  .group-chevron { font-size: 10px; width: 10px; }
  .group-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
  .group-label { flex: 1; }
  .group-count { background: var(--hover-btn); border-radius: 10px; padding: 0 6px; font-size: 10px; }

  .view-row {
    display: flex; align-items: center; gap: 3px; height: 28px;
    border-radius: 3px; cursor: default; user-select: none; position: relative;
    color: var(--text); border: 1px solid transparent;
    transition: background 0.06s;
  }
  .view-row:hover         { background: var(--hover); }
  .view-row.selected      { background: var(--selected); border-color: var(--accent-dim); }
  .view-row.completed     { opacity: 0.45; }
  .view-row:hover .row-actions { display: flex; }

  .row-actions {
    display: none; align-items: center; gap: 1px;
    position: absolute; right: 4px;
    background: var(--surface); border: 1px solid var(--border);
    border-radius: 4px; padding: 1px 2px;
  }

  .icon-btn {
    background: none; border: none; color: var(--text-dim); cursor: pointer;
    font-size: 11px; padding: 2px 3px; border-radius: 3px; line-height: 1; flex-shrink: 0;
    transition: color 0.1s, background 0.1s;
  }
  .icon-btn:hover { color: var(--text); background: var(--hover-btn); }
  .icon-btn.check:hover { color: var(--green); }
  .icon-btn.danger:hover { color: var(--red); }

  .flag-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; margin: 0 2px; }
  .flag-dot.empty { background: transparent; border: 1px solid var(--border); }

  .caption-cell { flex: 1; display: flex; align-items: center; gap: 4px; overflow: hidden; min-width: 0; }
  .caption-text {
    font-family: 'Cascadia Code', 'Fira Code', 'Consolas', monospace;
    font-size: 13px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; flex: 1; min-width: 0;
  }
  .caption-text.completed-text { text-decoration: line-through; color: var(--text-dim); }

  .caption-input {
    font-family: 'Cascadia Code', 'Fira Code', 'Consolas', monospace;
    font-size: 13px; background: var(--input-bg); border: 1px solid var(--accent);
    color: var(--text); padding: 1px 4px; border-radius: 3px; flex: 1; min-width: 0; outline: none;
  }

  .tag-chip { font-size: 10px; padding: 1px 5px; border-radius: 10px; border: 1px solid; white-space: nowrap; flex-shrink: 0; font-family: sans-serif; }
  .note-dot { font-size: 11px; color: var(--text-dim); flex-shrink: 0; }

  .col-date { font-size: 11px; color: var(--text-dim); width: 72px; text-align: right; flex-shrink: 0; font-family: sans-serif; }
  .col-date.overdue { color: var(--red); font-weight: 600; }
  .col-date.today   { color: var(--gold); font-weight: 600; }
  .col-date.soon    { color: var(--orange); }
</style>
