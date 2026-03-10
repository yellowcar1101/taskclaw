<script lang="ts">
  import TaskRow from './TaskRow.svelte';
  import {
    rootTasks, searchQuery,
    expandAll, collapseAll, clearSelection, createTask, editingId, showRapidInput
  } from '../stores/tasks';

  async function addRootTask() {
    const t = await createTask({ parent_id: null, caption: 'New task' });
    editingId.set(t.id);
  }
</script>

<!-- Toolbar -->
<div class="tree-toolbar">
  <button class="tb-btn primary" on:click={addRootTask}>+ Task</button>
  <button class="tb-btn rapid" on:click={() => showRapidInput.set(true)} title="Rapid Input (paste multiple tasks)">📋 Rapid</button>

  <input
    class="search-input"
    placeholder="Search…"
    bind:value={$searchQuery}
  />

  <div class="spacer"></div>

  <button class="tb-btn" on:click={expandAll} title="Expand all">⊞</button>
  <button class="tb-btn" on:click={collapseAll} title="Collapse all">⊟</button>
  <button class="tb-btn" on:click={clearSelection} title="Clear selection">✕</button>
</div>

<!-- Column headers -->
<div class="col-headers">
  <div class="col-spacer"></div>
  <div class="col-header caption-col">Task</div>
  <div class="col-header date-col">Start</div>
  <div class="col-header date-col">Due</div>
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
    font-family: sans-serif;
    transition: background 0.1s;
  }
  .tb-btn:hover { background: var(--hover); }
  .tb-btn.primary {
    background: var(--accent);
    color: #fff;
    border-color: var(--accent);
  }
  .tb-btn.primary:hover { filter: brightness(1.15); }
  .tb-btn.rapid { color: var(--text-dim); }

  .search-input {
    background: var(--input-bg);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 12px;
    width: 180px;
    outline: none;
  }
  .search-input:focus { border-color: var(--accent); }

  .spacer { flex: 1; }

  .col-headers {
    display: flex;
    align-items: center;
    gap: 3px;
    padding: 0 6px;
    height: 22px;
    background: var(--surface-elevated);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  /* toggle(14) + check(18) + flag-dot(12) + gaps ≈ 56px */
  .col-spacer { width: 56px; flex-shrink: 0; }
  .col-header {
    color: var(--text-dim);
    font-size: 10px;
    font-family: sans-serif;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 0 2px;
  }
  .col-header.caption-col { flex: 1; }
  .col-header.date-col { width: 72px; flex-shrink: 0; text-align: right; }

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
