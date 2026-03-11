<script lang="ts">
  import { writable } from 'svelte/store';
  import { onDestroy } from 'svelte';
  import TaskRow from './TaskRow.svelte';
  import {
    rootTasks, sortField, sortDir, toggleSort,
    expandAll, collapseAll, clearSelection, createTask, editingId,
    showRapidInput, selected, taskById, expanded,
    moveTask, reorderTasks, deleteTask, childrenOf,
    setSelected, outlineScrollToId, rangeAnchorId
  } from '../stores/tasks';
  import type { Task } from '../types';

  // ── Selected task context ─────────────────────────────────────────────────
  // For single select use that task; for multi-select use the range anchor (last singly-clicked task)
  $: selectedId = $selected.size === 1 ? [...$selected][0] : ($rangeAnchorId ?? null);
  $: selectedTask = selectedId ? ($taskById.get(selectedId) ?? null) : null;
  $: siblings = selectedTask
    ? ($childrenOf.get(selectedTask.parent_id ?? null) ?? [])
    : [];
  $: myIndex = selectedTask ? siblings.findIndex(s => s.id === selectedTask!.id) : -1;
  $: hasSelected = !!selectedTask;

  async function addRootTask() {
    // If a task is selected, create a sibling (same parent, positioned after it)
    if (selectedTask) {
      const t = await createTask({
        parent_id: selectedTask.parent_id ?? undefined,
        caption: 'New task',
        position: selectedTask.position + 0.5,
      });
      editingId.set(t.id);
    } else {
      const t = await createTask({ parent_id: null, caption: 'New task' });
      editingId.set(t.id);
    }
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
      if (e.key === 'F2' && selectedId) { e.preventDefault(); editingId.set(selectedId); }
    }
  }

  // ── Column visibility / order / widths ───────────────────────────────────
  const ALL_COLS = [
    { id: 'due',   label: 'Due',   sortKey: 'due_date',   defaultWidth: 80,  center: false },
    { id: 'start', label: 'Start', sortKey: 'start_date', defaultWidth: 70,  center: false },
    { id: 'flag',  label: 'Flag',  sortKey: null,         defaultWidth: 30,  center: true  },
    { id: 'tags',  label: 'Tags',  sortKey: null,         defaultWidth: 100, center: false },
  ];

  // Visibility
  const COL_VIS_KEY = 'visible_cols';
  function loadCols(): Set<string> {
    try { return new Set(JSON.parse(localStorage.getItem(COL_VIS_KEY) ?? '["due","start"]')); }
    catch { return new Set(['due', 'start']); }
  }
  let visibleCols = writable<Set<string>>(loadCols());
  const _unsubVis = visibleCols.subscribe(v => localStorage.setItem(COL_VIS_KEY, JSON.stringify([...v])));

  // Order
  const COL_ORDER_KEY = 'outline_col_order';
  function loadOrder(): string[] {
    try {
      const saved: string[] = JSON.parse(localStorage.getItem(COL_ORDER_KEY) ?? '[]');
      const ids = ALL_COLS.map(c => c.id);
      const merged = saved.filter(id => ids.includes(id));
      ids.forEach(id => { if (!merged.includes(id)) merged.push(id); });
      return merged;
    } catch { return ALL_COLS.map(c => c.id); }
  }
  let colOrder = writable<string[]>(loadOrder());
  const _unsubOrder = colOrder.subscribe(v => localStorage.setItem(COL_ORDER_KEY, JSON.stringify(v)));

  // Widths
  const COL_WIDTH_KEY = 'outline_col_widths';
  const DEFAULT_WIDTHS = Object.fromEntries(ALL_COLS.map(c => [c.id, c.defaultWidth]));
  function loadWidths(): Record<string, number> {
    try { return { ...DEFAULT_WIDTHS, ...JSON.parse(localStorage.getItem(COL_WIDTH_KEY) ?? '{}') }; }
    catch { return { ...DEFAULT_WIDTHS }; }
  }
  let colWidths = writable<Record<string, number>>(loadWidths());
  const _unsubWidths = colWidths.subscribe(v => localStorage.setItem(COL_WIDTH_KEY, JSON.stringify(v)));

  onDestroy(() => { _unsubVis(); _unsubOrder(); _unsubWidths(); });

  // Derived: visible cols in current order
  $: orderedVisibleCols = $colOrder
    .map(id => ALL_COLS.find(c => c.id === id)!)
    .filter(c => c && $visibleCols.has(c.id));

  // CSS vars string for the task-list container (widths + flex order)
  $: colVarsCss = orderedVisibleCols
    .map((c, i) => `--col-${c.id}-width:${$colWidths[c.id] ?? c.defaultWidth}px;--col-${c.id}-order:${i + 10}`)
    .join(';');

  // ── Column visibility toggle ──
  let showColMenu = false;
  function toggleCol(id: string) {
    visibleCols.update(s => { const n = new Set(s); n.has(id) ? n.delete(id) : n.add(id); return n; });
  }

  // ── Column resize ─────────────────────────────────────────────────────────
  let isResizing = false;

  function onResizeStart(e: MouseEvent, id: string) {
    e.preventDefault();
    e.stopPropagation();
    isResizing = true;
    const startX = e.clientX;
    const startW = $colWidths[id] ?? DEFAULT_WIDTHS[id] ?? 60;

    function onMove(e: MouseEvent) {
      const w = Math.max(30, startW + e.clientX - startX);
      colWidths.update(m => ({ ...m, [id]: w }));
    }
    function onUp() {
      isResizing = false;
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
    }
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  }

  // ── Column drag-to-reorder ────────────────────────────────────────────────
  let dragColId: string | null = null;
  let dragOverColId: string | null = null;

  function onColDragStart(e: DragEvent, id: string) {
    if (isResizing) { e.preventDefault(); return; }
    dragColId = id;
    e.dataTransfer!.effectAllowed = 'move';
  }
  function onColDragOver(e: DragEvent, id: string) {
    e.preventDefault();
    if (dragColId && dragColId !== id) dragOverColId = id;
  }
  function onColDragLeave() { dragOverColId = null; }
  function onColDrop(e: DragEvent, targetId: string) {
    e.preventDefault();
    if (dragColId && dragColId !== targetId) {
      colOrder.update(order => {
        const arr = [...order];
        const from = arr.indexOf(dragColId!);
        const to   = arr.indexOf(targetId);
        if (from !== -1 && to !== -1) { arr.splice(from, 1); arr.splice(to, 0, dragColId!); }
        return arr;
      });
    }
    dragColId = null; dragOverColId = null;
  }
  function onColDragEnd() { dragColId = null; dragOverColId = null; }
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
<div class="col-headers" class:col-resizing={isResizing}>
  <div class="col-spacer"></div>
  <button class="col-header caption-col" on:click={() => toggleSort('caption')}>
    Task
    {#if $sortField === 'caption'}<span class="sort-arrow">{$sortDir === 'asc' ? '↑' : '↓'}</span>{/if}
  </button>

  {#each orderedVisibleCols as col (col.id)}
    <div
      class="col-header resizable"
      class:center={col.center}
      class:drag-over={dragOverColId === col.id}
      style="width:{$colWidths[col.id] ?? col.defaultWidth}px"
      draggable={true}
      on:dragstart={e => onColDragStart(e, col.id)}
      on:dragover={e => onColDragOver(e, col.id)}
      on:dragleave={onColDragLeave}
      on:drop={e => onColDrop(e, col.id)}
      on:dragend={onColDragEnd}
      role="columnheader"
      tabindex="-1"
      title="Drag to reorder"
    >
      {#if col.sortKey}
        <button class="sort-inner" on:click={() => toggleSort(col.sortKey!)}>
          {col.label}
          {#if $sortField === col.sortKey}<span class="sort-arrow">{$sortDir === 'asc' ? '↑' : '↓'}</span>{/if}
        </button>
      {:else}
        <span class="col-label">{col.label}</span>
      {/if}
      <div class="resize-handle" role="separator" on:mousedown={e => onResizeStart(e, col.id)}></div>
    </div>
  {/each}
</div>

<!-- Task list (CSS vars drive column widths + order in rows) -->
<div class="task-list" role="none" style={colVarsCss}>
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
    padding: 0 6px;
    height: 22px;
    background: var(--surface-elevated);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    user-select: none;
  }
  .col-headers.col-resizing { cursor: col-resize; }

  .col-spacer { width: 46px; flex-shrink: 0; }

  .col-header {
    background: none;
    border: none;
    color: var(--text-dim);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 0;
    display: flex;
    align-items: center;
    border-radius: 3px;
    position: relative;
    flex-shrink: 0;
    transition: color 0.1s;
    overflow: visible;
  }
  .col-header.caption-col {
    flex: 1;
    cursor: pointer;
    padding: 0 4px;
    gap: 3px;
  }
  .col-header.caption-col:hover { color: var(--text); background: var(--hover); }

  /* Resizable column headers */
  .col-header.resizable { cursor: grab; }
  .col-header.resizable:hover { color: var(--text); background: var(--hover); }
  .col-header.resizable.drag-over { border-left: 2px solid var(--accent); }
  .col-header.center { justify-content: center; }

  /* Sort button inside resizable header */
  .sort-inner {
    background: none;
    border: none;
    color: inherit;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    cursor: pointer;
    padding: 0 4px;
    height: 100%;
    display: flex;
    align-items: center;
    gap: 3px;
    flex: 1;
    min-width: 0;
  }
  .col-label {
    padding: 0 4px;
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sort-arrow { color: var(--accent); font-size: 10px; }

  /* Resize handle — right edge of each resizable header */
  .resize-handle {
    position: absolute;
    right: -2px;
    top: 0;
    bottom: 0;
    width: 6px;
    cursor: col-resize;
    z-index: 2;
    border-radius: 3px;
  }
  .resize-handle:hover { background: var(--accent); opacity: 0.5; }

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
