<script lang="ts">
  import { createEventDispatcher, tick } from 'svelte';
  import type { Task } from '../types';
  import {
    expanded, selected, editingId, contextMenu,
    toggleExpanded, setSelected, setSelectedRange, updateTask,
    completeTask, createTask, reorderTasks,
    outlineScrollToId, flags, tags, childrenOf,
    themeFormatting,
  } from '../stores/tasks';
  import type { FormatKey } from '../stores/tasks';
  import { parseCaption } from '../parsing';
  import { dndzone } from 'svelte-dnd-action';

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
  $: if (isEditing) { editValue = editValue || task.caption; tick().then(() => inputEl?.focus()); }
  $: if (!isEditing) { editValue = ''; }
  $: children = isExpanded ? ($childrenOf.get(task.id) ?? []) : [];


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
      if (parsed.hideInViews !== undefined) updates.hide_in_views = parsed.hideInViews;
      if (parsed.subtasksInOrder !== undefined) updates.subtasks_in_order = parsed.subtasksInOrder;
      if (parsed.isProject !== undefined) updates.is_project = parsed.isProject;
      if (parsed.isFolder !== undefined) updates.is_folder = parsed.isFolder;
      if (parsed.colorHex !== undefined) updates.color = parsed.colorHex;
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
    if (e.shiftKey) {
      setSelectedRange(task.id);
    } else {
      setSelected(task.id, e.ctrlKey || e.metaKey);
    }
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

  function isStartOverdue(startDate: string | null): boolean {
    if (!startDate) return false;
    if (startDate.includes('T')) return new Date(startDate).getTime() < Date.now();
    return startDate < new Date().toISOString().slice(0, 10);
  }
  $: startOverdue = isStartOverdue(task.start_date);

  function formatReminder(val: string | null): string {
    if (!val) return '';
    const d = new Date(val);
    const months = ['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];
    const mon = months[d.getMonth()];
    const day = d.getDate();
    if (val.includes('T')) {
      const h = String(d.getHours()).padStart(2, '0');
      const m = String(d.getMinutes()).padStart(2, '0');
      return `${mon} ${day} ${h}:${m}`;
    }
    return `${mon} ${day}`;
  }

  function isReminderPast(val: string | null): boolean {
    if (!val || task.completed_at) return false;
    return new Date(val).getTime() < Date.now();
  }
  $: reminderPast = isReminderPast(task.reminder_at);

  // ── Theme Formatting ───────────────────────────────────────────────────────
  function taskFormatKey(t: typeof task): FormatKey {
    if (t.is_folder) return 'folder';
    if (t.is_project) return 'project';
    if (t.completed_at) return 'completed';
    if (t.hide_in_views) return 'hidden';
    return 'active';
  }

  const fontMap: Record<string, string> = {
    system:    'system-ui, -apple-system, sans-serif',
    segoe:     "'Segoe UI', system-ui, sans-serif",
    inter:     'Inter, system-ui, sans-serif',
    verdana:   'Verdana, Geneva, sans-serif',
    trebuchet: "'Trebuchet MS', Helvetica, sans-serif",
    calibri:   'Calibri, Candara, sans-serif',
    roboto:    'Roboto, system-ui, sans-serif',
    opensans:  "'Open Sans', system-ui, sans-serif",
    georgia:   'Georgia, serif',
    garamond:  "'Garamond', 'EB Garamond', serif",
    palatino:  "'Palatino Linotype', Palatino, serif",
    times:     "'Times New Roman', Times, serif",
    mono:      "'Cascadia Code', 'Fira Code', 'Consolas', monospace",
    consolas:  "Consolas, 'Courier New', monospace",
    courier:   "'Courier New', Courier, monospace",
  };

  $: fmt = $themeFormatting[taskFormatKey(task)];

  $: rowStyle = [
    fmt.bgColor ? `background:${fmt.bgColor}` : '',
    fmt.rowUnderlineColor ? `border-bottom:${fmt.rowUnderlineThickness}px solid ${fmt.rowUnderlineColor}` : '',
    fmt.fontFamily && fontMap[fmt.fontFamily] ? `font-family:${fontMap[fmt.fontFamily]}` : '',
  ].filter(Boolean).join(';');

  $: captionStyle = [
    fmt.fontColor ? `color:${fmt.fontColor}` : '',
    fmt.bold ? 'font-weight:700' : '',
    fmt.italic ? 'font-style:italic' : '',
    fmt.strikethrough ? 'text-decoration:line-through' : '',
    fmt.highlightColor ? `background:${fmt.highlightColor}` : '',
  ].filter(Boolean).join(';');

  // ── svelte-dnd-action: children list for this node ──────────────────────────
  const flipDurationMs = 150;

  // dndzone requires items with an `id` field — Task already has it
  $: dndChildren = isExpanded ? ($childrenOf.get(task.id) ?? []) : [];
  let localChildren: Task[] = [];
  $: localChildren = [...dndChildren];

  function handleDndConsider(e: CustomEvent<{ items: Task[] }>) {
    localChildren = e.detail.items;
  }

  async function handleDndFinalize(e: CustomEvent<{ items: Task[] }>) {
    localChildren = e.detail.items;
    const pairs: [string, number][] = localChildren.map((t, i) => [t.id, i + 1]);
    await reorderTasks(pairs);
  }
</script>

<div
  class="task-row"
  class:selected={isSelected}
  class:flash={flashing}
  style="padding-left: {depth * 20 + 6}px;{rowStyle}"
  bind:this={rowEl}
  on:click={onClick}
  on:dblclick={startEdit}
  on:contextmenu={onContextMenu}
  role="none"
  tabindex="-1"
>
  <!-- Sidebar color stripe -->
  {#if fmt.sidebarColor}
    <div class="sidebar-stripe" style="background:{fmt.sidebarColor}"></div>
  {/if}

  <!-- Drag handle (shown on hover, used by svelte-dnd-action) -->
  <span class="drag-handle" title="Drag to reorder">⠿</span>

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
    class="check-box"
    class:start-overdue={startOverdue}
    on:click|stopPropagation={onComplete}
    title="Mark complete"
    tabindex="-1"
  ></button>

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
      <span class="caption-text" class:starred={task.starred} style={captionStyle}>{task.caption}</span>
    {/if}

    <!-- Flag indicator (hidden when dedicated Flag column is visible) -->
    {#if task.flag && !visibleCols.has('flag')}
      <span class="flag-chip" style="background:{task.flag.color}22;color:{task.flag.color};border-color:{task.flag.color}44">
        {task.flag.name}
      </span>
    {/if}

    <!-- Tag chips (hidden when dedicated Tags column is visible) -->
    {#if !visibleCols.has('tags')}
      {#each task.tags as tag}
        <span class="tag-chip" style="background:{tag.color}22;color:{tag.color};border-color:{tag.color}44">
          {tag.name}
        </span>
      {/each}
    {/if}

    <!-- Recurrence indicator -->
    {#if task.recurrence_rule}
      <span class="recur-badge" title="Recurring">↻</span>
    {/if}

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

  <!-- Reminder column -->
  {#if visibleCols.has('reminder')}
    <div class="col-reminder" class:reminder-past={reminderPast}>{formatReminder(task.reminder_at)}</div>
  {/if}

  <!-- Flag column — dot only -->
  {#if visibleCols.has('flag')}
    <div class="col-flag">
      {#if task.flag}
        <span class="col-dot" style="background:{task.flag.color}" title={task.flag.name}></span>
      {/if}
    </div>
  {/if}

  <!-- Tags column — name labels -->
  {#if visibleCols.has('tags')}
    <div class="col-tags">
      {#each task.tags.slice(0, 3) as tag}
        <span class="col-tag-label" style="background:{tag.color}22;color:{tag.color};border-color:{tag.color}55" title={tag.name}>
          {tag.name}
        </span>
      {/each}
    </div>
  {/if}

</div>

<!-- Children (recursive) — wrapped in a dndzone for sibling reordering -->
{#if isExpanded && dndChildren.length > 0}
  <div
    use:dndzone={{ items: localChildren, flipDurationMs }}
    on:consider={handleDndConsider}
    on:finalize={handleDndFinalize}
    class="children-dnd-zone"
  >
    {#each localChildren as child (child.id)}
      <svelte:self task={child} depth={depth + 1} siblings={localChildren} {visibleCols} />
    {/each}
  </div>
{/if}

<style>
  .drag-handle {
    color: var(--text-dim);
    opacity: 0;
    font-size: 12px;
    cursor: grab;
    flex-shrink: 0;
    line-height: 1;
    padding: 0 2px;
    user-select: none;
    transition: opacity 0.1s;
  }
  .task-row:hover .drag-handle { opacity: 0.5; }
  .drag-handle:hover { opacity: 1 !important; }

  .children-dnd-zone {
    display: block;
  }

  .sidebar-stripe {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 3px;
    border-radius: 3px 0 0 3px;
    pointer-events: none;
  }

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
    margin: 0 2px;
  }
  .check-box:hover { border-color: var(--green); background: rgba(106,191,105,0.15); }
  .check-box.start-overdue { border-color: var(--red); background: rgba(255,255,255,0.06); }
  .check-box.start-overdue:hover { border-color: var(--red); background: rgba(224,92,92,0.15); }

  .caption-cell {
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

  .caption-input {
    font-family: var(--app-font);
    font-size: var(--app-font-size, 13px);
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

  .recur-badge {
    font-size: 11px;
    color: var(--accent);
    opacity: 0.7;
    flex-shrink: 0;
  }

  .email-badge {
    font-size: 11px;
    opacity: 0.6;
    flex-shrink: 0;
  }

  /* Column cells — widths and order driven by CSS variables set on the
     task-list container by TaskTree (so resize/reorder only needs parent update) */
  .col-due {
    font-size: 11px;
    color: var(--text-dim);
    width: var(--col-due-width, 80px);
    order: var(--col-due-order, 10);
    text-align: right;
    flex-shrink: 0;
  }
  .col-due.overdue { color: var(--red); font-weight: 600; }
  .col-due.today   { color: var(--amber); font-weight: 600; }
  .col-due.soon    { color: var(--amber); }

  .col-start {
    font-size: 11px;
    color: var(--text-dim);
    width: var(--col-start-width, 70px);
    order: var(--col-start-order, 11);
    text-align: right;
    flex-shrink: 0;
  }

  .col-reminder {
    font-size: 11px;
    color: var(--text-dim);
    width: var(--col-reminder-width, 90px);
    order: var(--col-reminder-order, 11);
    text-align: right;
    flex-shrink: 0;
  }
  .col-reminder.reminder-past { color: var(--red); }

  .col-flag {
    width: var(--col-flag-width, 30px);
    order: var(--col-flag-order, 12);
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .col-tags {
    width: var(--col-tags-width, 100px);
    order: var(--col-tags-order, 13);
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 3px;
    overflow: hidden;
  }
  .col-tag-label {
    font-size: 10px;
    padding: 1px 5px;
    border-radius: 8px;
    border: 1px solid;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 60px;
    flex-shrink: 0;
  }
  .col-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
    display: inline-block;
  }

</style>
