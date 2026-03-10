<script lang="ts">
  import TaskRow from './TaskRow.svelte';
  import {
    rootTasks, sortField, sortDir, toggleSort,
    filterContextId, contexts, searchQuery,
    expandAll, collapseAll, clearSelection, createTask, editingId
  } from '../stores/tasks';
  import type { SortField } from '../types';

  async function addRootTask() {
    const t = await createTask({ parent_id: null, caption: 'New task' });
    editingId.set(t.id);
  }

  function col(field: SortField, label: string) {
    return { field, label,
      active: $sortField === field,
      dir: $sortDir };
  }
</script>

<!-- Toolbar -->
<div class="tree-toolbar">
  <button class="tb-btn primary" on:click={addRootTask}>+ Task</button>

  <input
    class="search-input"
    placeholder="Search…"
    bind:value={$searchQuery}
  />

  <select class="ctx-filter" bind:value={$filterContextId}>
    <option value={null}>All contexts</option>
    {#each $contexts as ctx}
      <option value={ctx.id}>{ctx.name}</option>
    {/each}
  </select>

  <div class="spacer"></div>

  <button class="tb-btn" on:click={expandAll} title="Expand all">⊞</button>
  <button class="tb-btn" on:click={collapseAll} title="Collapse all">⊟</button>
  <button class="tb-btn" on:click={clearSelection} title="Clear selection">✕ sel</button>
</div>

<!-- Column headers -->
<div class="col-headers">
  <div class="col-spacer"></div><!-- toggle + check -->
  <button class="col-header caption-col" on:click={() => toggleSort('caption')}>
    Task
    {#if $sortField === 'caption'}<span class="sort-arrow">{$sortDir === 'asc' ? '↑' : '↓'}</span>{/if}
  </button>
  <button class="col-header due-col" on:click={() => toggleSort('due_date')}>
    Due
    {#if $sortField === 'due_date'}<span class="sort-arrow">{$sortDir === 'asc' ? '↑' : '↓'}</span>{/if}
  </button>
  <button class="col-header score-col" on:click={() => toggleSort('score')}>
    Score
    {#if $sortField === 'score'}<span class="sort-arrow">{$sortDir === 'asc' ? '↑' : '↓'}</span>{/if}
  </button>
  <button class="col-header iu-col" on:click={() => toggleSort('importance')}>
    I/U
    {#if $sortField === 'importance' || $sortField === 'urgency'}<span class="sort-arrow">{$sortDir === 'asc' ? '↑' : '↓'}</span>{/if}
  </button>
</div>

<!-- Task list -->
<div class="task-list" role="treegrid">
  {#each $rootTasks as task (task.id)}
    <TaskRow task={task} depth={0} siblings={$rootTasks} />
  {/each}

  {#if $rootTasks.length === 0}
    <div class="empty">No tasks. Click <strong>+ Task</strong> to add one.</div>
  {/if}
</div>

<style>
  .tree-toolbar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    background: var(--surface-elevated);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .tb-btn {
    background: var(--hover-btn);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 3px 10px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    transition: background 0.1s;
  }
  .tb-btn:hover { background: var(--hover); }
  .tb-btn.primary {
    background: var(--accent);
    color: #fff;
    border-color: var(--accent);
  }
  .tb-btn.primary:hover { background: var(--accent); }

  .search-input {
    background: var(--input-bg);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 12px;
    width: 160px;
    outline: none;
  }
  .search-input:focus { border-color: var(--accent); }

  .ctx-filter {
    background: var(--input-bg);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 3px 6px;
    border-radius: 4px;
    font-size: 12px;
    outline: none;
  }

  .spacer { flex: 1; }

  .col-headers {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 6px;
    height: 24px;
    background: var(--surface-elevated);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .col-spacer { width: 46px; flex-shrink: 0; }  /* toggle(16) + check(16) + gap */
  .col-header {
    background: none;
    border: none;
    color: var(--text-dim);
    font-size: 11px;
    font-family: sans-serif;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    cursor: pointer;
    padding: 0 2px;
    text-align: left;
    display: flex;
    align-items: center;
    gap: 3px;
    border-radius: 3px;
    transition: color 0.1s, background 0.1s;
  }
  .col-header:hover { color: var(--text); background: var(--hover); }
  .col-header.caption-col { flex: 1; }
  .col-header.due-col     { width: 80px; flex-shrink: 0; }
  .col-header.score-col   { width: 64px; flex-shrink: 0; }
  .col-header.iu-col      { width: 36px; flex-shrink: 0; }
  .sort-arrow { color: var(--accent); font-size: 10px; }

  .task-list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 2px;
  }

  .empty {
    text-align: center;
    color: var(--text-dim);
    padding: 60px 20px;
    font-size: 13px;
    font-family: sans-serif;
  }
</style>
