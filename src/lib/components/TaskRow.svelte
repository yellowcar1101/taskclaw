<script lang="ts">
  import { createEventDispatcher, tick } from 'svelte';
  import type { Task } from '../types';
  import {
    expanded, selected, editingId, contextMenu,
    toggleExpanded, setSelected, updateTask,
    completeTask, createTask,
    outlineScrollToId, flags, tags
  } from '../stores/tasks';
  import { parseCaption } from '../parsing';

  export let task: Task;
  export let depth: number = 0;
  export let siblings: Task[] = [];
  export let visibleCols: Set<string> = new Set(['due', 'start']);

  const dispatch = createEventDispatcher();

  let editValue = '';
  let inputEl: HTMLInputElement;
  let rowEl: HTMLDivElement;
  let flashing = false;

  $: isExpanded = $expanded.has(task.id);
  $: isSelected = $selected.has(task.id);
  $: isEditing = $editingId === task.id;
  $: if (isEditing) tick().then(() => inputEl?.focus());
  $: children = isExpanded ? getChildren(task.id) : [];


  // Scroll-flash: watch outlineScrollToId
  $: if ($outlineScrollToId === task.id) {
    outlineScrollToId.set(null);
    setTimeout(() => {
      rowEl?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
      flashing = true;
      setTimeout(() => flashing = false, 800);
    }, 50);
  }

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
    if (e.key === 'Enter' && e.altKey) {
      // Alt+Enter: apply inline parsing to current edit value
      e.preventDefault();
      const parsed = parseCaption(editValue, $flags, $tags);
      editValue = parsed.caption;
      const updates: Record<string, unknown> = { caption: parsed.caption };
      if (parsed.flagId) updates.flag_id = parsed.flagId;
      if (parsed.tagIds.length) updates.tag_ids = parsed.tagIds;
      if (parsed.starred) updates.starred = true;
      if (parsed.startDate) updates.start_date = parsed.startDate;
      if (parsed.dueDate) updates.due_date = parsed.dueDate;
      if (parsed.reminderAt) updates.reminder_at = parsed.reminderAt;
      updateTask(task.id, updates as any);
      editingId.set(null);
      return;
    }
    if (e.key === 'Enter') { e.preventDefault(); commitEdit(); }
    if (e.key === 'Escape') { editingId.set(null); }
    if (e.key === 'Tab') {
      e.preventDefault();
      commitEdit();
      if (!e.shiftKey) dispatch('addBelow', { parentId: task.parent_id, afterId: task.id });
    }
  }

  function onClick(e: MouseEvent) {
    setSelected(task.id, e.ctrlKey || e.metaKey);
  }

  function onContextMenu(e: MouseEvent) {
    e.preventDefault();
    contextMenu.set({ x: e.clientX, y: e.clientY, taskId: task.id });
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
    const diff = Math.floor((d.getTime() - Date.now()) / 86400000);
    if (diff < 0) return `${Math.abs(diff)}d overdue`;
    if (diff === 0) return 'Today';
    if (diff === 1) return 'Tomorrow';
    if (diff <= 7) return `${diff}d`;
    return d.toLocaleDateString('en-GB', { day: 'numeric', month: 'short' });
  }

  function dueClass(due: string | null): string {
    if (!due) return '';
    const diff = Math.floor((new Date(due).getTime() - Date.now()) / 86400000);
    if (diff < 0) return 'overdue';
    if (diff === 0) return 'today';
    if (diff <= 3) return 'soon';
    return '';
  }

  // Drag and drop
  let dragging = false;
  let dragOver = false;

  function onDragStart(e: DragEvent) {
    dragging = true;
    e.dataTransfer!.setData('taskId', task.id);
    e.dataTransfer!.effectAllowed = 'move';
  }
  function onDragEnd() { dragging = false; }
  function onDragOver(e: DragEvent) { e.preventDefault(); dragOver = true; e.dataTransfer!.dropEffect = 'move'; }
  function onDragLeave() { dragOver = false; }
  async function onDrop(e: DragEvent) {
    e.preventDefault(); dragOver = false;
    const srcId = e.dataTransfer!.getData('taskId');
    if (!srcId || srcId === task.id) return;
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
  class:flash={flashing}
  style="padding-left: {depth * 20 + 6}px"
  bind:this={rowEl}
  on:click={onClick}
  on:dblclick={startEdit}
  on:contextmenu={onContextMenu}
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

    <!-- Flag indicator -->
    {#if task.flag}
      <span class="flag-chip" style="background:{task.flag.color}22;color:{task.flag.color};border-color:{task.flag.color}44">
        {task.flag.name}
      </span>
    {/if}

    <!-- Tag chips -->
    {#each task.tags as tag}
      <span class="tag-chip" style="background:{tag.color}22;color:{tag.color};border-color:{tag.color}44">
        {tag.name}
      </span>
    {/each}

    <!-- Email link indicators -->
    {#each task.email_links as link}
      <span class="email-badge" title={link.subject ?? 'Email link'}>✉</span>
    {/each}
  </div>

  <!-- Due date -->
  {#if visibleCols.has('due')}
    <div class="col-due {dueClass(task.due_date)}">{formatDue(task.due_date)}</div>
  {/if}

  <!-- Start date -->
  {#if visibleCols.has('start')}
    <div class="col-start">{task.start_date ? task.start_date.split('T')[0] : ''}</div>
  {/if}

  <!-- Flag column -->
  {#if visibleCols.has('flag') && task.flag}
    <div class="col-flag">
      <span class="flag-pill" style="background:{task.flag.color}22;color:{task.flag.color};">{task.flag.name}</span>
    </div>
  {/if}

  <!-- Tags column -->
  {#if visibleCols.has('tags') && task.tags.length}
    <div class="col-tags">
      {#each task.tags.slice(0, 2) as tag}
        <span class="tag-pill" style="background:{tag.color}22;color:{tag.color};">{tag.name}</span>
      {/each}
    </div>
  {/if}

</div>

<!-- Children (recursive) -->
{#if isExpanded && children.length > 0}
  {#each children as child (child.id)}
    <svelte:self task={child} depth={depth + 1} siblings={children} {visibleCols} />
  {/each}
{/if}

<style>
  .drop-zone {
    height: 2px;
    margin: 0;
    transition: background 0.1s, height 0.1s;
  }
  .drop-zone.active { height: 4px; background: var(--accent); border-radius: 2px; }

  .task-row {
    display: flex;
    align-items: center;
    gap: 4px;
    height: var(--row-height, 28px);
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
  .task-row.flash { animation: row-flash 0.8s ease; }
  @keyframes row-flash {
    0%   { background: var(--accent-dim); }
    100% { background: transparent; }
  }

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
  .caption-text.starred::before { content: '★ '; color: var(--amber); }

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

  .flag-chip, .tag-chip {
    font-size: 10px;
    padding: 1px 5px;
    border-radius: 10px;
    border: 1px solid;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .email-badge {
    font-size: 11px;
    opacity: 0.6;
    flex-shrink: 0;
  }

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
    width: 90px;
    flex-shrink: 0;
    overflow: hidden;
  }
  .col-tags {
    width: 100px;
    flex-shrink: 0;
    display: flex;
    gap: 3px;
    overflow: hidden;
  }
  .flag-pill, .tag-pill {
    font-size: 10px;
    padding: 1px 5px;
    border-radius: 8px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: block;
    max-width: 100%;
  }

</style>
