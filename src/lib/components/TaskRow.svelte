<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { Task } from '../types';
  import {
    expanded, selected, editingId, detailTaskId,
    toggleExpanded, setSelected, updateTask, deleteTask,
    completeTask, moveTask, getChildren, reorderTasks, createTask, openDetail
  } from '../stores/tasks';

  export let task: Task;
  export let depth: number = 0;
  export let siblings: Task[] = [];

  const dispatch = createEventDispatcher();

  let hovered = false;
  let editValue = '';
  let inputEl: HTMLInputElement;

  $: isExpanded  = $expanded.has(task.id);
  $: isSelected  = $selected.has(task.id);
  $: isEditing   = $editingId === task.id;
  $: isDetail    = $detailTaskId === task.id;
  $: children    = isExpanded ? getChildren(task.id) : [];
  $: myIndex     = siblings.findIndex(s => s.id === task.id);
  $: isCompleted = !!task.completed_at;

  function startEdit() {
    editValue = task.caption;
    editingId.set(task.id);
    setTimeout(() => inputEl?.focus(), 10);
  }

  async function commitEdit() {
    if (editValue.trim() && editValue !== task.caption)
      await updateTask(task.id, { caption: editValue.trim() });
    editingId.set(null);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') { e.preventDefault(); commitEdit(); }
    if (e.key === 'Escape') { editingId.set(null); }
  }

  function onClick(e: MouseEvent) {
    setSelected(task.id, e.ctrlKey || e.metaKey);
  }

  async function onIndent() {
    const prev = siblings[myIndex - 1];
    if (!prev) return;
    const prevChildren = getChildren(prev.id);
    const newPos = prevChildren.length > 0 ? prevChildren[prevChildren.length - 1].position + 1000 : 1000;
    expanded.update(s => { const n = new Set(s); n.add(prev.id); return n; });
    await moveTask(task.id, prev.id, newPos);
  }

  async function onOutdent() {
    if (!task.parent_id) return;
    await moveTask(task.id, null, task.position + 0.5);
  }

  async function onMoveUp() {
    if (myIndex <= 0) return;
    await reorderTasks([[task.id, siblings[myIndex - 1].position - 0.5]]);
  }

  async function onMoveDown() {
    if (myIndex >= siblings.length - 1) return;
    await reorderTasks([[task.id, siblings[myIndex + 1].position + 0.5]]);
  }

  async function onDelete() {
    if (confirm(`Delete "${task.caption}"${task.has_children ? ' and all subtasks' : ''}?`))
      await deleteTask(task.id);
  }

  async function addSubtask() {
    const t = await createTask({ parent_id: task.id, caption: 'New task' });
    expanded.update(s => { const n = new Set(s); n.add(task.id); return n; });
    editingId.set(t.id);
  }

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
    const dt = new Date(d + 'T00:00:00');
    const diff = Math.floor((dt.getTime() - Date.now()) / 86400000);
    if (diff < 0)  return 'overdue';
    if (diff === 0) return 'today';
    if (diff <= 3)  return 'soon';
    return '';
  }

  // Drag reorder
  let dragging = false;
  let dragOver = false;

  function onDragStart(e: DragEvent) {
    dragging = true;
    e.dataTransfer!.setData('taskId', task.id);
    e.dataTransfer!.effectAllowed = 'move';
  }
  function onDragEnd()  { dragging = false; }
  function onDragOver(e: DragEvent) { e.preventDefault(); dragOver = true; }
  function onDragLeave() { dragOver = false; }
  async function onDrop(e: DragEvent) {
    e.preventDefault(); dragOver = false;
    const srcId = e.dataTransfer!.getData('taskId');
    if (!srcId || srcId === task.id) return;
    await reorderTasks([[srcId, task.position - 0.5]]);
  }
</script>

<div class="drop-zone" class:active={dragOver}
  on:dragover={onDragOver} on:dragleave={onDragLeave} on:drop={onDrop} role="none"></div>

<div
  class="task-row"
  class:selected={isSelected}
  class:detail-open={isDetail}
  class:dragging
  class:completed={isCompleted}
  style="padding-left: {depth * 20 + 4}px"
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
  <!-- Expand toggle -->
  <button class="icon-btn toggle" class:invisible={!task.has_children}
    on:click|stopPropagation={() => toggleExpanded(task.id)} tabindex="-1">
    {isExpanded ? '▾' : '▸'}
  </button>

  <!-- Complete -->
  <button class="icon-btn check" on:click|stopPropagation={() => completeTask(task.id, !isCompleted)}
    title={isCompleted ? 'Reopen' : 'Complete'} tabindex="-1">
    {isCompleted ? '●' : '○'}
  </button>

  <!-- Flag dot -->
  {#if task.flag}
    <span class="flag-dot" style="background:{task.flag.color}" title={task.flag.name}></span>
  {:else}
    <span class="flag-dot empty"></span>
  {/if}

  <!-- Caption -->
  <div class="caption-cell">
    {#if isEditing}
      <input bind:this={inputEl} bind:value={editValue} class="caption-input"
        on:blur={commitEdit} on:keydown={onKeydown} />
    {:else}
      <span class="caption-text" class:starred={task.starred} class:completed-text={isCompleted}>
        {task.caption}
      </span>
    {/if}

    <!-- Tags -->
    {#each task.tags as tag}
      <span class="tag-chip" style="background:{tag.color}22;color:{tag.color};border-color:{tag.color}55">
        {tag.name}
      </span>
    {/each}

    <!-- Email indicators -->
    {#each task.email_links as link}
      <a class="email-badge" href={link.link_type === 'gmail'
        ? `https://mail.google.com/mail/u/0/#search/${link.link_data}`
        : link.link_data}
        title={link.subject ?? 'Open email'} on:click|stopPropagation>
        {link.link_type === 'gmail' ? '✉' : '📧'}
      </a>
    {/each}

    <!-- Note indicator -->
    {#if task.note}
      <span class="note-dot" title="Has notes">¶</span>
    {/if}
  </div>

  <!-- Start date -->
  <div class="col-date {dateClass(task.start_date)}" title="Start date">
    {formatDate(task.start_date)}
  </div>

  <!-- Due date -->
  <div class="col-date {dateClass(task.due_date)}" title="Due date">
    {formatDate(task.due_date)}
  </div>

  <!-- Hover actions -->
  {#if hovered || isSelected}
    <div class="actions" on:click|stopPropagation role="none">
      <button class="icon-btn" on:click={() => openDetail(task.id)} title="Open detail">⋯</button>
      <button class="icon-btn" on:click={addSubtask} title="Add subtask">+</button>
      <button class="icon-btn" on:click={onIndent} title="Indent" disabled={myIndex === 0}>⇥</button>
      <button class="icon-btn" on:click={onOutdent} title="Outdent" disabled={!task.parent_id}>⇤</button>
      <button class="icon-btn" on:click={onMoveUp} title="Move up" disabled={myIndex === 0}>↑</button>
      <button class="icon-btn" on:click={onMoveDown} title="Move down" disabled={myIndex >= siblings.length - 1}>↓</button>
      <button class="icon-btn danger" on:click={onDelete} title="Delete">✕</button>
    </div>
  {/if}
</div>

<!-- Children -->
{#if isExpanded && children.length > 0}
  {#each children as child (child.id)}
    <svelte:self task={child} depth={depth + 1} siblings={children} />
  {/each}
{/if}

<style>
  .drop-zone { height: 2px; transition: background 0.1s, height 0.1s; }
  .drop-zone.active { height: 4px; background: var(--accent); border-radius: 2px; }

  .task-row {
    display: flex;
    align-items: center;
    gap: 3px;
    height: 28px;
    border-radius: 3px;
    cursor: default;
    user-select: none;
    position: relative;
    color: var(--text);
    border: 1px solid transparent;
    transition: background 0.06s;
  }
  .task-row:hover    { background: var(--hover); }
  .task-row.selected { background: var(--selected); border-color: var(--accent-dim); }
  .task-row.detail-open { border-color: var(--accent-dim); }
  .task-row.dragging { opacity: 0.4; }
  .task-row.completed { opacity: 0.45; }

  .icon-btn {
    background: none; border: none; color: var(--text-dim); cursor: pointer;
    font-size: 11px; padding: 2px 3px; border-radius: 3px; line-height: 1; flex-shrink: 0;
    transition: color 0.1s, background 0.1s;
  }
  .icon-btn:hover:not(:disabled) { color: var(--text); background: var(--hover-btn); }
  .icon-btn:disabled  { opacity: 0.2; cursor: default; }
  .icon-btn.danger:hover { color: var(--red); }
  .icon-btn.invisible { visibility: hidden; }
  .icon-btn.toggle    { font-size: 10px; width: 14px; text-align: center; }
  .icon-btn.check     { font-size: 12px; }
  .icon-btn.check:hover { color: var(--green); }

  .flag-dot {
    width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0;
    margin: 0 2px;
  }
  .flag-dot.empty { background: transparent; border: 1px solid var(--border); }

  .caption-cell {
    flex: 1; display: flex; align-items: center; gap: 4px;
    overflow: hidden; min-width: 0;
  }

  .caption-text {
    font-family: 'Cascadia Code', 'Fira Code', 'Consolas', monospace;
    font-size: 13px; white-space: nowrap; overflow: hidden;
    text-overflow: ellipsis; flex: 1; min-width: 0;
  }
  .caption-text.starred::before { content: '★ '; color: var(--gold); }
  .caption-text.completed-text  { text-decoration: line-through; color: var(--text-dim); }

  .caption-input {
    font-family: 'Cascadia Code', 'Fira Code', 'Consolas', monospace;
    font-size: 13px; background: var(--input-bg); border: 1px solid var(--accent);
    color: var(--text); padding: 1px 4px; border-radius: 3px; flex: 1; min-width: 0; outline: none;
  }

  .tag-chip {
    font-size: 10px; padding: 1px 5px; border-radius: 10px; border: 1px solid;
    white-space: nowrap; flex-shrink: 0; font-family: sans-serif;
  }

  .email-badge { font-size: 11px; text-decoration: none; opacity: 0.7; flex-shrink: 0; }
  .email-badge:hover { opacity: 1; }

  .note-dot { font-size: 11px; color: var(--text-dim); flex-shrink: 0; }

  .col-date {
    font-size: 11px; color: var(--text-dim); width: 72px;
    text-align: right; flex-shrink: 0; font-family: sans-serif;
  }
  .col-date.overdue { color: var(--red); font-weight: 600; }
  .col-date.today   { color: var(--gold); font-weight: 600; }
  .col-date.soon    { color: var(--orange); }

  .actions {
    display: flex; align-items: center; gap: 1px;
    position: absolute; right: 4px;
    background: var(--surface); border: 1px solid var(--border);
    border-radius: 4px; padding: 1px 2px;
  }
</style>
