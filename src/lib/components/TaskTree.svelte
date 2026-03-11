<script lang="ts">
  import { writable } from 'svelte/store';
  import TaskRow from './TaskRow.svelte';
  import {
    rootTasks, sortField, sortDir, toggleSort,
    expandAll, collapseAll, clearSelection, createTask, editingId,
    showRapidInput, selected, taskById, expanded,
    moveTask, reorderTasks, deleteTask, childrenOf,
    setSelected, outlineScrollToId
  } from '../stores/tasks';
  import type { Task } from '../types';

  // ── Selected task context ─────────────────────────────────────────────────
  $: selectedId = $selected.size === 1 ? [...$selected][0] : null;
  $: selectedTask = selectedId ? ($taskById.get(selectedId) ?? null) : null;
  $: siblings = selectedTask
    ? ($childrenOf.get(selectedTask.parent_id ?? null) ?? [])
    : [];
  $: myIndex = selectedTask ? siblings.findIndex(s => s.id === selectedTask!.id) : -1;
  $: hasSelected = !!selectedTask;

  async function addRootTask() {
    const t = await createTask({ parent_id: null, caption: 'New task' });
    editingId.set(t.id);
  }

  async function addSubtask() {
    if (!selectedTask) return;
    const t = await createTask({ parent_id: selectedTask.id, caption: 'New task' });
    expanded.update(s => { const n = new Set(s); n.add(selectedTask!.id); return n; });
    editingId.set(t.id);
  }

  async function onIndent() {
    if (!selectedTask || myIndex <= 0) return;
    const prev = siblings[myIndex - 1];
    const prevChildren = $childrenOf.get(prev.id) ?? [];
    const newPos = prevChildren.length > 0
      ? prevChildren[prevChildren.length - 1].position + 1000
      : 1000;
    expanded.update(s => { const n = new Set(s); n.add(prev.id); return n; });
    await moveTask(selectedTask.id, prev.id, newPos);
  }

  async function onOutdent() {
    if (!selectedTask?.parent_id) return;
    const parent = $taskById.get(selectedTask.parent_id);
    await moveTask(selectedTask.id, parent?.parent_id ?? null, selectedTask.position + 0.5);
  }

  async function onMoveUp() {
    if (!selectedTask || myIndex <= 0) return;
    const prev = siblings[myIndex - 1];
    await reorderTasks([[selectedTask.id, prev.position - 0.5]]);
  }

  async function onMoveDown() {
    if (!selectedTask || myIndex >= siblings.length - 1) return;
    const next = siblings[myIndex + 1];
    await reorderTasks([[selectedTask.id, next.position + 0.5]]);
  }

  async function onDelete() {
    if (!selectedTask) return;
    const t = selectedTask;
    if (confirm(`Delete "${t.caption}"${t.has_children ? ' and all subtasks' : ''}?`)) {
      await deleteTask(t.id);
    }
  }

  // ── Arrow key navigation ─────────────────────────────────────────────────
  function getVisibleFlat(): string[] {
    const result: string[] = [];
    function walk(tasks: Task[]) {
      for (const t of tasks) {
        result.push(t.id);
        if ($expanded.has(t.id)) {
          walk($childrenOf.get(t.id) ?? []);
        }
      }
    }
    walk($rootTasks);
    return result;
  }

  function navigateSelection(dir: 1 | -1) {
    const flat = getVisibleFlat();
    if (!flat.length) return;
    const idx = selectedId ? flat.indexOf(selectedId) : -1;
    const newIdx = idx === -1
      ? (dir === 1 ? 0 : flat.length - 1)
      : Math.max(0, Math.min(flat.length - 1, idx + dir));
    const newId = flat[newIdx];
    if (newId) {
      setSelected(newId, false);
      outlineScrollToId.set(newId);
    }
  }

  function onGlobalKeydown(e: KeyboardEvent) {
    const inInput = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || (e.target as HTMLElement)?.isContentEditable;
    if (e.ctrlKey && e.shiftKey && e.key === 'I') {
      e.preventDefault();
      showRapidInput.set(true);
    }
    if (!inInput) {
      if (e.key === 'ArrowDown') { e.preventDefault(); navigateSelection(1); }
      if (e.key === 'ArrowUp')   { e.preventDefault(); navigateSelection(-1); }
      if (e.key === 'Delete' && hasSelected) onDelete();
    }
  }

  // ── Column visibility ─────────────────────────────────────────────────────
  const COL_KEY = 'visible_cols';
  function loadCols(): Set<string> {
    try { return new Set(JSON.parse(localStorage.getItem(COL_KEY) ?? '["due","start"]')); }
    catch { return new Set(['due', 'start']); }
  }
  let visibleCols = writable<Set<string>>(loadCols());
  visibleCols.subscribe(v => localStorage.setItem(COL_KEY, JSON.stringify([...v])));

  let showColMenu = false;
  const ALL_COLS = [
    { id: 'due',   label: 'Due Date' },
    { id: 'start', label: 'Start Date' },
    { id: 'flag',  label: 'Flag' },
    { id: 'tags',  label: 'Tags' },
  ];
  function toggleCol(id: string) {
    visibleCols.update(s => {
      const n = new Set(s);
      n.has(id) ? n.delete(id) : n.add(id);
      return n;
    });
  }
</script>

<svelte:window on:keydown={onGlobalKeydown} />

<!-- Toolbar -->
<div class="tree-toolbar">
  <button tabindex="-1" class="tb-btn primary" on:click={addRootTask}>+ Task</button>
  <button tabindex="-1" class="tb-btn" on:click={() => showRapidInput.set(true)} title="Rapid Input (Ctrl+Shift+I)">📋 Rapid</button>

  <div class="tb-divider"></div>

  <!-- Contextual task actions -->
  <button tabindex="-1" class="tb-btn" on:click={addSubtask} disabled={!hasSelected} title="Add subtask to selected">+ Sub</button>
  <button tabindex="-1" class="tb-btn icon" on:click={onIndent}   disabled={!hasSelected || myIndex <= 0}             title="Indent (→)">⇥</button>
  <button tabindex="-1" class="tb-btn icon" on:click={onOutdent}  disabled={!hasSelected || !selectedTask?.parent_id} title="Outdent (←)">⇤</button>
  <button tabindex="-1" class="tb-btn icon" on:click={onMoveUp}   disabled={!hasSelected || myIndex <= 0}             title="Move up">↑</button>
  <button tabindex="-1" class="tb-btn icon" on:click={onMoveDown} disabled={!hasSelected || myIndex >= siblings.length - 1} title="Move down">↓</button>
  <button tabindex="-1" class="tb-btn icon danger" on:click={onDelete} disabled={!hasSelected} title="Delete selected (Del)">✕</button>

  <div class="spacer"></div>

  <!-- Column visibility -->
  <div style="position:relative">
    <button tabindex="-1" class="tb-btn" on:click={() => showColMenu = !showColMenu} title="Show/hide columns">
      Columns ▾
    </button>
    {#if showColMenu}
      <div class="col-menu" role="menu">
        {#each ALL_COLS as col}
          <label class="col-menu-item">
            <input type="checkbox" checked={$visibleCols.has(col.id)}
              on:change={() => toggleCol(col.id)} />
            {col.label}
          </label>
        {/each}
      </div>
    {/if}
  </div>

  <button tabindex="-1" class="tb-btn icon" on:click={expandAll}    title="Expand all">⊞</button>
  <button tabindex="-1" class="tb-btn icon" on:click={collapseAll}  title="Collapse all">⊟</button>
  <button tabindex="-1" class="tb-btn icon" on:click={clearSelection} title="Clear selection">○</button>
</div>

{#if showColMenu}
  <div class="col-menu-backdrop" on:click={() => showColMenu = false} role="none"></div>
{/if}

<!-- Column headers -->
<div class="col-headers">
  <div class="col-spacer"></div>
  <button class="col-header caption-col" on:click={() => toggleSort('caption')}>
    Task
    {#if $sortField === 'caption'}<span class="sort-arrow">{$sortDir === 'asc' ? '↑' : '↓'}</span>{/if}
  </button>
  {#if $visibleCols.has('due')}
    <button class="col-header due-col" on:click={() => toggleSort('due_date')}>
      Due
      {#if $sortField === 'due_date'}<span class="sort-arrow">{$sortDir === 'asc' ? '↑' : '↓'}</span>{/if}
    </button>
  {/if}
  {#if $visibleCols.has('start')}
    <button class="col-header start-col" on:click={() => toggleSort('start_date')}>
      Start
      {#if $sortField === 'start_date'}<span class="sort-arrow">{$sortDir === 'asc' ? '↑' : '↓'}</span>{/if}
    </button>
  {/if}
  {#if $visibleCols.has('flag')}
    <div class="col-header flag-col">Flag</div>
  {/if}
  {#if $visibleCols.has('tags')}
    <div class="col-header tags-col">Tags</div>
  {/if}
</div>

<!-- Task list -->
<div class="task-list" role="none">
  {#each $rootTasks as task (task.id)}
    <TaskRow task={task} depth={0} siblings={$rootTasks} visibleCols={$visibleCols} />
  {/each}

  {#if $rootTasks.length === 0}
    <div class="empty">No tasks. Click <strong>+ Task</strong> to add one.</div>
  {/if}
</div>

<style>
  .tree-toolbar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    background: var(--surface-elevated);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    flex-wrap: wrap;
  }

  .tb-btn {
    background: var(--hover-btn);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 3px 8px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    transition: background 0.1s;
    white-space: nowrap;
  }
  .tb-btn.icon { padding: 3px 6px; min-width: 24px; }
  .tb-btn:hover:not(:disabled) { background: var(--hover); }
  .tb-btn:disabled { opacity: 0.3; cursor: default; }
  .tb-btn.primary { background: var(--accent); color: #fff; border-color: var(--accent); }
  .tb-btn.primary:hover:not(:disabled) { opacity: 0.9; }
  .tb-btn.danger:hover:not(:disabled) { color: var(--red); border-color: var(--red); }

  .tb-divider { width: 1px; height: 18px; background: var(--border); margin: 0 2px; flex-shrink: 0; }
  .spacer { flex: 1; }

  .col-menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.4);
    z-index: 500;
    padding: 6px 0;
    min-width: 140px;
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
  .col-menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: 499;
  }

  .col-headers {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 6px;
    height: 22px;
    background: var(--surface-elevated);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .col-spacer { width: 46px; flex-shrink: 0; }
  .col-header {
    background: none;
    border: none;
    color: var(--text-dim);
    font-size: 11px;
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
  .col-header.start-col   { width: 70px; flex-shrink: 0; }
  .col-header.flag-col    { width: 30px; flex-shrink: 0; text-align: center; justify-content: center; }
  .col-header.tags-col    { width: 52px; flex-shrink: 0; text-align: center; justify-content: center; }
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
  }
</style>
