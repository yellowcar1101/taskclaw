<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { Task } from '../types';
  import {
    expanded, selected, editingId,
    toggleExpanded, setSelected, updateTask, deleteTask,
    completeTask, moveTask, getChildren, reorderTasks, createTask
  } from '../stores/tasks';

  export let task: Task;
  export let depth: number = 0;
  export let siblings: Task[] = [];

  const dispatch = createEventDispatcher();

  let hovered = false;
  let editValue = '';
  let inputEl: HTMLInputElement;

  $: isExpanded = $expanded.has(task.id);
  $: isSelected = $selected.has(task.id);
  $: isEditing = $editingId === task.id;
  $: children = isExpanded ? getChildren(task.id) : [];
  $: myIndex = siblings.findIndex(s => s.id === task.id);

  function startEdit() {
    editValue = task.caption;
    editingId.set(task.id);
    setTimeout(() => inputEl?.focus(), 10);
  }

  async function commitEdit() {
    if (editValue.trim() && editValue !== task.caption) {
      await updateTask(task.id, { caption: editValue.trim() });
    }
    editingId.set(null);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') { e.preventDefault(); commitEdit(); }
    if (e.key === 'Escape') { editingId.set(null); }
    if (e.key === 'Tab') {
      e.preventDefault();
      commitEdit();
      if (!e.shiftKey) dispatch('addBelow', { parentId: task.parent_id, afterId: task.id });
    }
  }

  function onClick(e: MouseEvent) {
    setSelected(task.id, e.ctrlKey || e.metaKey, e.shiftKey);
  }

  async function onIndent() {
    const prev = siblings[myIndex - 1];
    if (!prev) return;
    const prevChildren = getChildren(prev.id);
    const newPos = prevChildren.length > 0
      ? prevChildren[prevChildren.length - 1].position + 1000
      : 1000;
    expanded.update(s => { const n = new Set(s); n.add(prev.id); return n; });
    await moveTask(task.id, prev.id, newPos);
  }

  async function onOutdent() {
    if (!task.parent_id) return;
    const grandparent = siblings.find(s => s.id === task.parent_id)?.parent_id ?? null;
    // Find parent's position among its siblings to insert after it
    await moveTask(task.id, grandparent, task.position + 0.5);
  }

  async function onMoveUp() {
    if (myIndex <= 0) return;
    const prev = siblings[myIndex - 1];
    await reorderTasks([
      [task.id, prev.position - 0.5],
    ]);
  }

  async function onMoveDown() {
    if (myIndex >= siblings.length - 1) return;
    const next = siblings[myIndex + 1];
    await reorderTasks([
      [task.id, next.position + 0.5],
    ]);
  }

  async function onDelete() {
    if (confirm(`Delete "${task.caption}"${task.has_children ? ' and all subtasks' : ''}?`)) {
      await deleteTask(task.id);
    }
  }

  async function onComplete() {
    await completeTask(task.id, true);
  }

  async function addSubtask() {
    const t = await createTask({ parent_id: task.id, caption: 'New task' });
    expanded.update(s => { const n = new Set(s); n.add(task.id); return n; });
    editingId.set(t.id);
  }

  function formatDue(due: string | null): string {
    if (!due) return '';
    const d = new Date(due);
    const now = new Date();
    const diff = Math.floor((d.getTime() - now.getTime()) / 86400000);
    if (diff < 0) return `${Math.abs(diff)}d overdue`;
    if (diff === 0) return 'Today';
    if (diff === 1) return 'Tomorrow';
    if (diff <= 7) return `${diff}d`;
    return d.toLocaleDateString('en-GB', { day: 'numeric', month: 'short' });
  }

  function dueClass(due: string | null): string {
    if (!due) return '';
    const d = new Date(due);
    const diff = Math.floor((d.getTime() - Date.now()) / 86400000);
    if (diff < 0) return 'overdue';
    if (diff === 0) return 'today';
    if (diff <= 3) return 'soon';
    return '';
  }

  // Drag and drop (reorder within same parent)
  let dragging = false;
  let dragOver = false;

  function onDragStart(e: DragEvent) {
    dragging = true;
    e.dataTransfer!.setData('taskId', task.id);
    e.dataTransfer!.setData('parentId', task.parent_id ?? '');
    e.dataTransfer!.effectAllowed = 'move';
  }
  function onDragEnd() { dragging = false; }

  function onDragOver(e: DragEvent) {
    e.preventDefault();
    dragOver = true;
    e.dataTransfer!.dropEffect = 'move';
  }
  function onDragLeave() { dragOver = false; }

  async function onDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    const srcId = e.dataTransfer!.getData('taskId');
    if (!srcId || srcId === task.id) return;
    // Drop on a row = insert before that row
    await reorderTasks([[srcId, task.position - 0.5]]);
  }
</script>

<!-- Drop zone above row -->
<div
  class="drop-zone"
  class:active={dragOver}
  on:dragover={onDragOver}
  on:dragleave={onDragLeave}
  on:drop={onDrop}
  role="none"
></div>

<div
  class="task-row"
  class:selected={isSelected}
  class:dragging
  style="padding-left: {depth * 20 + 6}px"
  on:click={onClick}
  on:dblclick={startEdit}
  on:mouseenter={() => hovered = true}
  on:mouseleave={() => hovered = false}
  draggable={true}
  on:dragstart={onDragStart}
  on:dragend={onDragEnd}
  role="row"
  tabindex="0"
>
  <!-- Expand/collapse toggle -->
  <button
    class="icon-btn toggle"
    class:invisible={!task.has_children}
    on:click|stopPropagation={() => toggleExpanded(task.id)}
    title={isExpanded ? 'Collapse' : 'Expand'}
    tabindex="-1"
  >
    {isExpanded ? '▾' : '▸'}
  </button>

  <!-- Complete checkbox -->
  <button
    class="icon-btn check"
    on:click|stopPropagation={onComplete}
    title="Complete"
    tabindex="-1"
  >○</button>

  <!-- Caption -->
  <div class="caption-cell">
    {#if isEditing}
      <input
        bind:this={inputEl}
        bind:value={editValue}
        class="caption-input"
        on:blur={commitEdit}
        on:keydown={onKeydown}
      />
    {:else}
      <span class="caption-text" class:starred={task.starred}>{task.caption}</span>
    {/if}

    <!-- Context chips -->
    {#each task.contexts as ctx}
      <span class="ctx-chip" style="background:{ctx.color}22;color:{ctx.color};border-color:{ctx.color}44">
        {ctx.name}
      </span>
    {/each}

    <!-- Email link indicator -->
    {#each task.email_links as link}
      <a
        class="email-badge"
        href={link.link_type === 'gmail'
          ? `https://mail.google.com/mail/u/0/#search/${link.link_data}`
          : link.link_data}
        title={link.subject ?? 'Open email'}
        on:click|stopPropagation
      >
        {link.link_type === 'gmail' ? '✉' : '📧'}
      </a>
    {/each}
  </div>

  <!-- Due date -->
  <div class="col-due {dueClass(task.due_date)}">
    {formatDue(task.due_date)}
  </div>

  <!-- Score badge -->
  <div class="col-score" title="Priority score: {task.score.toFixed(0)}">
    <div class="score-bar" style="width:{task.score}%"></div>
    <span class="score-num">{task.score.toFixed(0)}</span>
  </div>

  <!-- I/U badges -->
  <div class="col-iu">
    <span class="iu-badge" title="Importance">{task.importance}</span>
    <span class="iu-badge" title="Urgency">{task.urgency}</span>
  </div>

  <!-- Hover action buttons -->
  {#if hovered || isSelected}
    <div class="actions" on:click|stopPropagation role="none">
      <button class="icon-btn" on:click={addSubtask} title="Add subtask">+</button>
      <button class="icon-btn" on:click={onIndent} title="Indent (make child of row above)" disabled={myIndex === 0}>⇥</button>
      <button class="icon-btn" on:click={onOutdent} title="Outdent" disabled={!task.parent_id}>⇤</button>
      <button class="icon-btn" on:click={onMoveUp} title="Move up" disabled={myIndex === 0}>↑</button>
      <button class="icon-btn" on:click={onMoveDown} title="Move down" disabled={myIndex >= siblings.length - 1}>↓</button>
      <button class="icon-btn danger" on:click={onDelete} title="Delete">✕</button>
    </div>
  {/if}
</div>

<!-- Children (recursive) -->
{#if isExpanded && children.length > 0}
  {#each children as child (child.id)}
    <svelte:self task={child} depth={depth + 1} siblings={children} />
  {/each}
{/if}

<style>
  .drop-zone {
    height: 2px;
    margin: 0;
    transition: background 0.1s, height 0.1s;
  }
  .drop-zone.active {
    height: 4px;
    background: var(--accent);
    border-radius: 2px;
  }

  .task-row {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 30px;
    border-radius: 3px;
    cursor: default;
    user-select: none;
    position: relative;
    color: var(--text);
    border: 1px solid transparent;
    transition: background 0.08s;
  }
  .task-row:hover { background: var(--hover); }
  .task-row.selected { background: var(--selected); border-color: var(--accent-dim); }
  .task-row.dragging { opacity: 0.4; }

  .icon-btn {
    background: none;
    border: none;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 11px;
    padding: 2px 4px;
    border-radius: 3px;
    line-height: 1;
    flex-shrink: 0;
    transition: color 0.1s, background 0.1s;
  }
  .icon-btn:hover:not(:disabled) { color: var(--text); background: var(--hover-btn); }
  .icon-btn:disabled { opacity: 0.25; cursor: default; }
  .icon-btn.danger:hover { color: var(--red); }
  .icon-btn.invisible { visibility: hidden; }
  .icon-btn.toggle { font-size: 10px; width: 16px; text-align: center; }
  .icon-btn.check { font-size: 12px; color: var(--text-dim); }
  .icon-btn.check:hover { color: var(--green); }

  .caption-cell {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 5px;
    overflow: hidden;
    min-width: 0;
  }

  .caption-text {
    font-family: 'Cascadia Code', 'Fira Code', 'Consolas', monospace;
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    min-width: 0;
  }
  .caption-text.starred::before { content: '★ '; color: var(--gold); }

  .caption-input {
    font-family: 'Cascadia Code', 'Fira Code', 'Consolas', monospace;
    font-size: 13px;
    background: var(--input-bg);
    border: 1px solid var(--accent);
    color: var(--text);
    padding: 1px 4px;
    border-radius: 3px;
    flex: 1;
    min-width: 0;
    outline: none;
  }

  .ctx-chip {
    font-size: 10px;
    padding: 1px 5px;
    border-radius: 10px;
    border: 1px solid;
    white-space: nowrap;
    flex-shrink: 0;
    font-family: sans-serif;
  }

  .email-badge {
    font-size: 11px;
    text-decoration: none;
    opacity: 0.7;
    flex-shrink: 0;
  }
  .email-badge:hover { opacity: 1; }

  .col-due {
    font-size: 11px;
    color: var(--text-dim);
    width: 80px;
    text-align: right;
    flex-shrink: 0;
    font-family: sans-serif;
  }
  .col-due.overdue { color: var(--red); font-weight: 600; }
  .col-due.today   { color: var(--gold); font-weight: 600; }
  .col-due.soon    { color: var(--orange); }

  .col-score {
    width: 64px;
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }
  .score-bar {
    height: 4px;
    border-radius: 2px;
    background: var(--accent);
    opacity: 0.6;
    transition: width 0.3s;
  }
  .score-num {
    font-size: 10px;
    color: var(--text-dim);
    width: 20px;
    text-align: right;
    font-family: sans-serif;
  }

  .col-iu {
    width: 36px;
    display: flex;
    gap: 2px;
    flex-shrink: 0;
  }
  .iu-badge {
    font-size: 10px;
    color: var(--text-dim);
    font-family: sans-serif;
    width: 14px;
    text-align: center;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 1px;
    position: absolute;
    right: 4px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 1px 2px;
  }
</style>
