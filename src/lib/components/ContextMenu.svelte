<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import {
    contextMenu, allTasks, flags, tags, taskById,
    createTask, updateTask, deleteTask, completeTask,
    detailTaskId, navigateToOutline, expanded, editingId
  } from '../stores/tasks';
  import { api } from '../api';
  import type { Task } from '../types';

  let menuEl: HTMLDivElement;

  $: task = $contextMenu ? $taskById.get($contextMenu.taskId) : null;
  $: x = $contextMenu?.x ?? 0;
  $: y = $contextMenu?.y ?? 0;

  function close() { contextMenu.set(null); }

  function reposition() {
    if (!menuEl || !$contextMenu) return;
    const vw = window.innerWidth, vh = window.innerHeight;
    const rect = menuEl.getBoundingClientRect();
    if (rect.right > vw) menuEl.style.left = `${vw - rect.width - 4}px`;
    if (rect.bottom > vh) menuEl.style.top = `${vh - rect.height - 4}px`;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
  }

  function clickOutside(e: MouseEvent) {
    if (menuEl && !menuEl.contains(e.target as Node)) close();
  }

  onMount(() => {
    setTimeout(reposition, 0);
    window.addEventListener('keydown', onKeydown);
    window.addEventListener('mousedown', clickOutside, true);
    window.addEventListener('scroll', close, true);
  });
  onDestroy(() => {
    window.removeEventListener('keydown', onKeydown);
    window.removeEventListener('mousedown', clickOutside, true);
    window.removeEventListener('scroll', close, true);
  });

  // ── Actions ─────────────────────────────────────────────────────────────────

  async function act(fn: () => Promise<unknown>) {
    await fn();
    close();
  }

  async function newTask() {
    if (!task) return;
    const parentId = task.parent_id;
    close();
    const t = await createTask({ parent_id: parentId, caption: 'New task' });
    editingId.set(t.id);
  }
  async function newSubtask() {
    if (!task) return;
    const taskId = task.id;
    close();
    const t = await createTask({ parent_id: taskId, caption: 'New task' });
    // Expand the parent so the new subtask is visible
    expanded.update(s => { const n = new Set(s); n.add(taskId); return n; });
    editingId.set(t.id);
  }
  async function newProject() {
    if (!task) return;
    const parentId = task.parent_id;
    close();
    const t = await createTask({ parent_id: parentId, caption: 'New project', is_project: true });
    editingId.set(t.id);
  }
  async function newFolder() {
    if (!task) return;
    const parentId = task.parent_id;
    close();
    const t = await createTask({ parent_id: parentId, caption: 'New folder', is_folder: true });
    editingId.set(t.id);
  }

  async function doComplete() {
    if (!task) return;
    act(() => completeTask(task!.id, true));
  }

  async function doCompleteBranch() {
    if (!task) return;
    act(async () => {
      await api.completeBranch(task!.id, true);
      const tasks = await api.getAllFlat();
      allTasks.set(tasks);
    });
  }

  async function doUncompleteBranch() {
    if (!task) return;
    act(async () => {
      await api.completeBranch(task!.id, false);
      const tasks = await api.getAllFlat();
      allTasks.set(tasks);
    });
  }

  async function doDuplicate() {
    if (!task) return;
    act(async () => {
      const newTask = await api.duplicateTask(task!.id);
      allTasks.update(ts => [...ts, newTask]);
    });
  }

  async function doDelete() {
    if (!task) return;
    const hasChildren = task.has_children;
    close();
    if (hasChildren && !confirm(`Delete "${task.caption}" and all subtasks?`)) return;
    await deleteTask(task.id);
  }

  async function doSkipOccurrence() {
    if (!task) return;
    act(async () => {
      const updated = await api.skipOccurrence(task!.id);
      allTasks.update(ts => ts.map(t => t.id === task!.id ? updated : t));
    });
  }

  async function setFlag(flagId: string | null) {
    if (!task) return;
    act(() => updateTask(task!.id, { flag_id: flagId ?? '' }));
  }

  async function toggleTag(tagId: string) {
    if (!task) return;
    const current = task.tags.map(t => t.id);
    const next = current.includes(tagId)
      ? current.filter(id => id !== tagId)
      : [...current, tagId];
    act(() => updateTask(task!.id, { tag_ids: next }));
  }

  async function clearTags() {
    if (!task) return;
    act(() => updateTask(task!.id, { tag_ids: [] }));
  }

  async function doStar(starred: boolean) {
    if (!task) return;
    act(() => updateTask(task!.id, { starred }));
  }

  function setDate(field: 'start_date' | 'due_date', expr: string | null) {
    if (!task) return;
    const val = expr ?? '';
    act(() => updateTask(task!.id, { [field]: val }));
  }

  function quickDate(n: number): string {
    const d = new Date();
    d.setDate(d.getDate() + n);
    return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}`;
  }

  function weekdayLabel(n: number): string {
    const d = new Date();
    d.setDate(d.getDate() + n);
    return d.toLocaleDateString('en-US', { weekday: 'short', month: 'short', day: 'numeric' });
  }

  // Submenu state
  let activeSubmenu: string | null = null;
  let submenuDateField: 'start_date' | 'due_date' = 'due_date';

  function openSubmenu(name: string) { activeSubmenu = name; }
  function closeSubmenu(name: string) { if (activeSubmenu === name) activeSubmenu = null; }

  async function copyCaptionTree() {
    if (!task) return;
    const all = get(allTasks);
    const map = new Map<string | null, Task[]>();
    for (const t of all) {
      const k = t.parent_id ?? null;
      if (!map.has(k)) map.set(k, []);
      map.get(k)!.push(t);
    }
    function build(id: string, depth: number): string {
      const task = all.find(t => t.id === id)!;
      const children = (map.get(id) ?? []).map(c => build(c.id, depth + 1));
      return ['  '.repeat(depth) + task.caption, ...children].join('\n');
    }
    const text = build(task.id, 0);
    close();
    await navigator.clipboard.writeText(text).catch(() => {});
  }

  // Sort subtasks
  let showSortDialog = false;
  let sortBy = 'caption';
  let sortDirLocal = 'asc';

  async function doSortSubtasks() {
    if (!task) return;
    close();
    await api.sortSubtasks(task.id, sortBy, sortDirLocal);
    const tasks = await api.getAllFlat();
    allTasks.set(tasks);
  }
</script>

{#if $contextMenu && task}
  <div
    class="ctx-menu"
    bind:this={menuEl}
    style="left:{x}px;top:{y}px"
    role="menu"
  >
    <div class="item" role="menuitem" tabindex="0" on:click={newTask} on:keydown>New Task</div>
    <div class="item" role="menuitem" tabindex="0" on:click={newSubtask} on:keydown>New Subtask</div>
    <div class="item" role="menuitem" tabindex="0" on:click={newProject} on:keydown>New Project</div>
    <div class="item" role="menuitem" tabindex="0" on:click={newFolder} on:keydown>New Folder</div>
    <div class="sep"></div>

    <!-- Set Start Date submenu -->
    <div class="item has-sub" role="menuitem" tabindex="0"
      on:mouseenter={() => { submenuDateField = 'start_date'; openSubmenu('start'); }}
      on:mouseleave={() => closeSubmenu('start')}
      on:keydown
    >
      Set Start Date ▶
      {#if activeSubmenu === 'start'}
        <div class="submenu" on:mouseenter={() => openSubmenu('start')} on:mouseleave={() => closeSubmenu('start')} role="menu" tabindex="-1">
          {#each [0,1,2,3,4,5,6] as n}
            <div class="item" role="menuitem" tabindex="0" on:click={() => setDate('start_date', quickDate(n))} on:keydown>
              {n === 0 ? 'Today' : n === 1 ? 'Tomorrow' : `In ${n} days`} &nbsp;<span class="dim">{weekdayLabel(n)}</span>
            </div>
          {/each}
          <div class="sep"></div>
          <div class="item" role="menuitem" tabindex="0" on:click={() => setDate('start_date', null)} on:keydown>Clear</div>
        </div>
      {/if}
    </div>

    <!-- Set Due Date submenu -->
    <div class="item has-sub" role="menuitem" tabindex="0"
      on:mouseenter={() => { submenuDateField = 'due_date'; openSubmenu('due'); }}
      on:mouseleave={() => closeSubmenu('due')}
      on:keydown
    >
      Set Due Date ▶
      {#if activeSubmenu === 'due'}
        <div class="submenu" on:mouseenter={() => openSubmenu('due')} on:mouseleave={() => closeSubmenu('due')} role="menu" tabindex="-1">
          {#each [0,1,2,3,4,5,6] as n}
            <div class="item" role="menuitem" tabindex="0" on:click={() => setDate('due_date', quickDate(n))} on:keydown>
              {n === 0 ? 'Today' : n === 1 ? 'Tomorrow' : `In ${n} days`} &nbsp;<span class="dim">{weekdayLabel(n)}</span>
            </div>
          {/each}
          <div class="sep"></div>
          <div class="item" role="menuitem" tabindex="0" on:click={() => setDate('due_date', task?.start_date ?? null)} on:keydown>Set Equal to Start Date</div>
          <div class="sep"></div>
          <div class="item" role="menuitem" tabindex="0" on:click={() => setDate('due_date', null)} on:keydown>Clear</div>
        </div>
      {/if}
    </div>

    {#if task.recurrence_rule}
      <div class="item" role="menuitem" tabindex="0" on:click={doSkipOccurrence} on:keydown>Skip Occurrence</div>
    {/if}
    <div class="sep"></div>

    <div class="item" role="menuitem" tabindex="0" on:click={doDuplicate} on:keydown>Duplicate Task</div>
    <div class="item" role="menuitem" tabindex="0" on:click={copyCaptionTree} on:keydown>Copy Tasks as Text</div>
    <div class="sep"></div>

    <!-- Flag submenu -->
    <div class="item has-sub" role="menuitem" tabindex="0"
      on:mouseenter={() => openSubmenu('flag')}
      on:mouseleave={() => closeSubmenu('flag')}
      on:keydown
    >
      Flag ▶
      {#if activeSubmenu === 'flag'}
        <div class="submenu" on:mouseenter={() => openSubmenu('flag')} on:mouseleave={() => closeSubmenu('flag')} role="menu" tabindex="-1">
          {#each $flags as flag}
            <div class="item" role="menuitem" tabindex="0" on:click={() => setFlag(task?.flag_id === flag.id ? null : flag.id)} on:keydown>
              <span class="flag-dot" style="background:{flag.color}"></span>
              {flag.name}
              {#if task.flag_id === flag.id}<span class="check">✓</span>{/if}
            </div>
          {/each}
          <div class="sep"></div>
          <div class="item" role="menuitem" tabindex="0" on:click={() => setFlag(null)} on:keydown>Clear Flag</div>
        </div>
      {/if}
    </div>

    <!-- Tag submenu -->
    <div class="item has-sub" role="menuitem" tabindex="0"
      on:mouseenter={() => openSubmenu('tag')}
      on:mouseleave={() => closeSubmenu('tag')}
      on:keydown
    >
      Tag ▶
      {#if activeSubmenu === 'tag'}
        <div class="submenu" on:mouseenter={() => openSubmenu('tag')} on:mouseleave={() => closeSubmenu('tag')} role="menu" tabindex="-1">
          {#each $tags as tag}
            <div class="item" role="menuitem" tabindex="0" on:click={() => toggleTag(tag.id)} on:keydown>
              <span class="tag-dot" style="background:{tag.color}"></span>
              {tag.name}
              {#if task.tags.some(t => t.id === tag.id)}<span class="check">✓</span>{/if}
            </div>
          {/each}
          <div class="sep"></div>
          <div class="item" role="menuitem" tabindex="0" on:click={clearTags} on:keydown>Clear All Tags</div>
        </div>
      {/if}
    </div>

    <!-- Star submenu -->
    <div class="item has-sub" role="menuitem" tabindex="0"
      on:mouseenter={() => openSubmenu('star')}
      on:mouseleave={() => closeSubmenu('star')}
      on:keydown
    >
      Star ▶
      {#if activeSubmenu === 'star'}
        <div class="submenu" on:mouseenter={() => openSubmenu('star')} on:mouseleave={() => closeSubmenu('star')} role="menu" tabindex="-1">
          <div class="item" role="menuitem" tabindex="0" on:click={() => doStar(true)} on:keydown>⭐ Star Task</div>
          <div class="item" role="menuitem" tabindex="0" on:click={() => doStar(false)} on:keydown>Clear Star</div>
          <div class="item" role="menuitem" tabindex="0" on:click={() => doStar(!task?.starred)} on:keydown>Toggle Star</div>
        </div>
      {/if}
    </div>

    <div class="sep"></div>

    <!-- Advanced submenu -->
    <div class="item has-sub" role="menuitem" tabindex="0"
      on:mouseenter={() => openSubmenu('adv')}
      on:mouseleave={() => closeSubmenu('adv')}
      on:keydown
    >
      Advanced ▶
      {#if activeSubmenu === 'adv'}
        <div class="submenu" on:mouseenter={() => openSubmenu('adv')} on:mouseleave={() => closeSubmenu('adv')} role="menu" tabindex="-1">
          <div class="item" role="menuitem" tabindex="0" on:click={doCompleteBranch} on:keydown>Complete Task and All Subtasks</div>
          <div class="item" role="menuitem" tabindex="0" on:click={doUncompleteBranch} on:keydown>Uncomplete Task and All Subtasks</div>
        </div>
      {/if}
    </div>

    <div class="sep"></div>
    <div class="item danger" role="menuitem" tabindex="0" on:click={doDelete} on:keydown>🗑 Delete Task</div>
  </div>
{/if}

<style>
  .ctx-menu {
    position: fixed;
    z-index: 9000;
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.4);
    padding: 4px 0;
    min-width: 200px;
    font-size: 12px;
    color: var(--text);
  }

  .item {
    padding: 5px 12px;
    cursor: pointer;
    white-space: nowrap;
    display: flex;
    align-items: center;
    gap: 6px;
    position: relative;
    border-radius: 3px;
    margin: 0 4px;
  }
  .item:hover { background: var(--hover); }
  .item.danger { color: var(--red); }
  .item.has-sub { padding-right: 24px; }
  .dim { color: var(--text-dim); font-size: 11px; }
  .check { margin-left: auto; color: var(--accent); }

  .sep {
    height: 1px;
    background: var(--border);
    margin: 3px 0;
  }

  .submenu {
    position: absolute;
    left: 100%;
    top: -4px;
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.4);
    padding: 4px 0;
    min-width: 200px;
    z-index: 9001;
  }

  .flag-dot, .tag-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
</style>
