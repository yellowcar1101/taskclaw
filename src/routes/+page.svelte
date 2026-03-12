<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import '../app.css';
  import TaskTree from '$lib/components/TaskTree.svelte';
  import TaskDetail from '$lib/components/TaskDetail.svelte';
  import ContextMenu from '$lib/components/ContextMenu.svelte';
  import RapidInput from '$lib/components/RapidInput.svelte';
  import ReminderWindow from '$lib/components/ReminderWindow.svelte';
  import Prefs from '$lib/components/Prefs.svelte';
  import PlanView from '$lib/components/PlanView.svelte';
  import ViewSettingsDialog from '$lib/components/ViewSettingsDialog.svelte';
  import {
    loadAll, detailTaskId, showPrefs, showRapidInput,
    contextMenu, views, activeTabId, editingViewId,
    searchQuery
  } from '$lib/stores/tasks';

  let ready = false;

  // ── File menu ──────────────────────────────────────────────────────────────
  let fileMenuOpen = false;
  let currentFile = '';
  let fileError = '';

  async function loadCurrentFile() {
    const { invoke } = await import('@tauri-apps/api/core');
    currentFile = await invoke<string>('file_current_path').catch(() => '');
  }

  function currentFileName(path: string): string {
    if (!path) return 'tasks.db';
    return path.split(/[\\/]/).pop() ?? path;
  }

  async function fileNew() {
    fileMenuOpen = false;
    fileError = '';
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const { invoke } = await import('@tauri-apps/api/core');
      const path = await save({
        title: 'New TaskClaw database',
        filters: [{ name: 'TaskClaw DB', extensions: ['db'] }],
        defaultPath: 'tasks.db',
      });
      if (!path) return;
      await invoke('file_new', { newPath: path });
      currentFile = path;
      await loadAll();
    } catch (e: any) {
      fileError = e?.message ?? String(e);
    }
  }

  async function fileOpen() {
    fileMenuOpen = false;
    fileError = '';
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const { invoke } = await import('@tauri-apps/api/core');
      const path = await open({
        title: 'Open TaskClaw database',
        filters: [{ name: 'TaskClaw DB', extensions: ['db'] }],
        multiple: false,
      });
      if (!path) return;
      await invoke('file_open', { newPath: path as string });
      currentFile = path as string;
      await loadAll();
    } catch (e: any) {
      fileError = e?.message ?? String(e);
    }
  }

  async function fileSaveAs() {
    fileMenuOpen = false;
    fileError = '';
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const { invoke } = await import('@tauri-apps/api/core');
      const path = await save({
        title: 'Save database as…',
        filters: [{ name: 'TaskClaw DB', extensions: ['db'] }],
        defaultPath: currentFileName(currentFile),
      });
      if (!path) return;
      await invoke('file_save_as', { dest: path });
      currentFile = path;
    } catch (e: any) {
      fileError = e?.message ?? String(e);
    }
  }

  // ── Startup ────────────────────────────────────────────────────────────────
  onMount(async () => {
    const root = document.documentElement;
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
    const savedFont = localStorage.getItem('app_font') ?? 'system';
    const savedSize = localStorage.getItem('app_font_size') ?? '12';
    const savedCompact = localStorage.getItem('app_compact') === 'true';
    root.style.setProperty('--app-font', fontMap[savedFont] ?? fontMap.system);
    root.style.setProperty('--app-font-size', savedSize + 'px');
    root.style.setProperty('--row-height', savedCompact ? '22px' : '28px');
    const savedTaskColor = localStorage.getItem('app_task_color') ?? '';
    if (savedTaskColor) root.style.setProperty('--task-color', savedTaskColor);

    await loadAll();
    await loadCurrentFile();
    ready = true;

    function onBeforeUnload() {
      if (localStorage.getItem('startup_remember_position') === 'true') {
        import('@tauri-apps/api/core').then(({ invoke }) => invoke('save_window_state').catch(() => {}));
      }
    }
    window.addEventListener('beforeunload', onBeforeUnload);
    return () => window.removeEventListener('beforeunload', onBeforeUnload);
  });

  async function addView() {
    const { api } = await import('$lib/api');
    const v = await api.createView({
      name: 'New View', show_completed: false, group_by: 'none',
      sort_by: 'position', sort_dir: 'asc', visible_fields: [],
      filter_json: '{"action_filter":"active"}'
    });
    views.update(arr => [...arr, v]);
    activeTabId.set(v.id);
    editingViewId.set(v.id);
  }

  // ── Svelte action: auto-focus element ────────────────────────────────────────
  function autoFocus(node: HTMLElement) { setTimeout(() => node.focus(), 0); }

  // ── Tab context menu ────────────────────────────────────────────────────────
  let tabCtxMenu: { x: number; y: number; viewId: string } | null = null;
  let tabRenaming: string | null = null; // viewId being renamed
  let tabRenameValue = '';

  function onTabContextMenu(e: MouseEvent, viewId: string) {
    e.preventDefault();
    tabCtxMenu = { x: e.clientX, y: e.clientY, viewId };
  }

  function closeTabCtx() { tabCtxMenu = null; }

  function startTabRename(viewId: string) {
    closeTabCtx();
    const v = $views.find(v => v.id === viewId);
    if (!v) return;
    tabRenaming = viewId;
    tabRenameValue = v.name;
  }

  async function commitTabRename() {
    if (!tabRenaming || !tabRenameValue.trim()) { tabRenaming = null; return; }
    const { api } = await import('$lib/api');
    const v = $views.find(v => v.id === tabRenaming);
    if (v) {
      const updated = await api.updateView(v.id, { ...v, name: tabRenameValue.trim() });
      views.update(vs => vs.map(x => x.id === v.id ? updated : x));
    }
    tabRenaming = null;
  }

  async function deleteView(viewId: string) {
    closeTabCtx();
    const { api } = await import('$lib/api');
    await api.deleteView(viewId);
    views.update(vs => vs.filter(v => v.id !== viewId));
    if ($activeTabId === viewId) activeTabId.set('outline');
  }
</script>

<div class="app-shell">
  <!-- Titlebar -->
  <header class="titlebar">

    <!-- File menu (top-left, subtle) -->
    <div class="file-menu-wrap">
      <button
        class="file-menu-btn"
        class:open={fileMenuOpen}
        on:click={() => { fileMenuOpen = !fileMenuOpen; fileError = ''; }}
        title="File"
        tabindex="-1"
      >≡</button>

      {#if fileMenuOpen}
        <div class="fm-backdrop" on:click={() => fileMenuOpen = false} role="none"></div>
        <div class="fm-panel" role="menu">
          <button class="fm-item" on:click={fileNew} role="menuitem">
            <span class="fm-icon">📄</span> New file…
          </button>
          <button class="fm-item" on:click={fileOpen} role="menuitem">
            <span class="fm-icon">📂</span> Open file…
          </button>
          <button class="fm-item" on:click={fileSaveAs} role="menuitem">
            <span class="fm-icon">💾</span> Save as…
          </button>
          <div class="fm-divider"></div>
          <div class="fm-current" title={currentFile}>
            <span class="fm-icon">🗄</span>
            <span class="fm-filename">{currentFileName(currentFile)}</span>
          </div>
          {#if fileError}
            <div class="fm-error">{fileError}</div>
          {/if}
        </div>
      {/if}
    </div>

    <div class="spacer"></div>
    {#if ready}
      <input class="search-input" placeholder="Search…" bind:value={$searchQuery} />
    {/if}
    <button class="titlebar-btn" on:click={() => showPrefs.set(true)} title="Preferences">⚙ Prefs</button>
  </header>

  <!-- View tabs -->
  {#if ready}
    <nav class="view-tabs">
      <button
        class="vtab" class:active={$activeTabId === 'outline'}
        on:click={() => activeTabId.set('outline')}
        tabindex="-1"
      >☰ Outline</button>
      <button
        class="vtab" class:active={$activeTabId === '__starred__'}
        on:click={() => activeTabId.set('__starred__')}
        tabindex="-1"
      >⭐ Starred</button>
      <button
        class="vtab" class:active={$activeTabId === '__today__'}
        on:click={() => activeTabId.set('__today__')}
        tabindex="-1"
      >◷ Today</button>

      {#each $views as view (view.id)}
        <div class="vtab-wrap" class:active={$activeTabId === view.id}
          on:contextmenu={e => onTabContextMenu(e, view.id)}
        >
          {#if tabRenaming === view.id}
            <input
              class="vtab-rename-input"
              bind:value={tabRenameValue}
              on:blur={commitTabRename}
              on:keydown={e => { if (e.key === 'Enter') commitTabRename(); if (e.key === 'Escape') tabRenaming = null; }}
              use:autoFocus
            />
          {:else}
            <button
              class="vtab" class:active={$activeTabId === view.id}
              on:click={() => activeTabId.set(view.id)}
              tabindex="-1"
            >{view.name}</button>
          {/if}
          <button
            class="vtab-gear"
            on:click|stopPropagation={() => editingViewId.set(view.id)}
            title="View settings"
            tabindex="-1"
          >⚙</button>
        </div>
      {/each}

      <button class="vtab vtab-add" on:click={addView} tabindex="-1" title="New view">+</button>
    </nav>
  {/if}

  <!-- Main area -->
  <div class="main-area">
    <div class="content">
      {#if !ready}
        <div class="loading">Loading…</div>
      {:else if $activeTabId === 'outline'}
        <TaskTree />
      {:else if $activeTabId === '__starred__' || $activeTabId === '__today__'}
        {#each [{ id: $activeTabId, name: $activeTabId === '__starred__' ? 'Starred' : 'Due Today',
                  show_completed: false, group_by: 'none', sort_by: 'due_date', sort_dir: 'asc',
                  visible_fields: [],
                  filter_json: $activeTabId === '__starred__' ? '{"action_filter":"all","starred":true}' : '{"action_filter":"all"}',
                  position: 0 }] as v}
          <PlanView view={v} />
        {/each}
      {:else}
        {#each $views as view (view.id)}
          {#if view.id === $activeTabId}
            <PlanView {view} />
          {/if}
        {/each}
      {/if}
    </div>

    {#if $detailTaskId}
      <TaskDetail />
    {/if}
  </div>
</div>

<!-- Overlays -->
{#if $contextMenu}
  <ContextMenu />
{/if}

{#if $showRapidInput}
  <RapidInput />
{/if}

{#if $showPrefs}
  <Prefs />
{/if}

{#if $editingViewId}
  <ViewSettingsDialog viewId={$editingViewId} on:close={() => editingViewId.set(null)} />
{/if}

<ReminderWindow />

{#if tabCtxMenu}
  <div class="tab-ctx-backdrop" on:click={closeTabCtx} role="none"></div>
  <div class="tab-ctx-menu" style="left:{tabCtxMenu.x}px;top:{tabCtxMenu.y}px" role="menu">
    <button class="tab-ctx-item" on:click={() => startTabRename(tabCtxMenu!.viewId)} role="menuitem">Rename</button>
    <button class="tab-ctx-item" on:click={() => editingViewId.set(tabCtxMenu!.viewId)} role="menuitem">Settings</button>
    <div class="tab-ctx-sep"></div>
    <button class="tab-ctx-item danger" on:click={() => deleteView(tabCtxMenu!.viewId)} role="menuitem">Delete</button>
  </div>
{/if}

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
    background: var(--bg);
  }

  .titlebar {
    display: flex;
    align-items: center;
    height: 36px;
    padding: 0 8px 0 4px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    -webkit-app-region: drag;
    user-select: none;
    gap: 6px;
  }

  /* ── File menu ── */
  .file-menu-wrap {
    position: relative;
    flex-shrink: 0;
    -webkit-app-region: no-drag;
  }

  .file-menu-btn {
    background: none;
    border: none;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 16px;
    line-height: 1;
    padding: 4px 8px;
    border-radius: 4px;
    transition: color 0.1s, background 0.1s;
  }
  .file-menu-btn:hover, .file-menu-btn.open { color: var(--text); background: var(--hover); }

  .fm-backdrop {
    position: fixed;
    inset: 0;
    z-index: 899;
  }

  .fm-panel {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    z-index: 900;
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.45);
    padding: 4px 0;
    min-width: 180px;
  }

  .fm-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    background: none;
    border: none;
    color: var(--text);
    font-size: 12px;
    padding: 6px 14px;
    cursor: pointer;
    text-align: left;
  }
  .fm-item:hover { background: var(--hover); }

  .fm-icon { font-size: 13px; flex-shrink: 0; }

  .fm-divider { height: 1px; background: var(--border); margin: 4px 0; }

  .fm-current {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 14px;
    font-size: 11px;
    color: var(--text-dim);
    overflow: hidden;
  }
  .fm-filename {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .fm-error {
    font-size: 11px;
    color: var(--red);
    padding: 4px 14px 6px;
    line-height: 1.4;
  }

  /* ── View tabs ── */
  .view-tabs {
    display: flex;
    align-items: stretch;
    gap: 0;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .view-tabs::-webkit-scrollbar { display: none; }

  .vtab {
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--text-dim);
    padding: 5px 14px;
    font-size: 12px;
    cursor: pointer;
    white-space: nowrap;
    flex-shrink: 0;
    transition: color 0.1s;
    margin-bottom: -1px;
  }
  .vtab:hover { color: var(--text); background: var(--hover); }
  .vtab.active { color: var(--accent); border-bottom-color: var(--accent); }

  .vtab-wrap {
    display: flex;
    align-items: stretch;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
  }
  .vtab-wrap.active { border-bottom-color: var(--accent); }
  .vtab-wrap .vtab { border-bottom: none; margin-bottom: 0; padding-right: 6px; }
  .vtab-wrap:hover { background: var(--hover); }

  .vtab-gear {
    background: none;
    border: none;
    color: transparent;
    cursor: pointer;
    font-size: 10px;
    padding: 0 6px 0 2px;
    flex-shrink: 0;
    transition: color 0.1s;
  }
  .vtab-wrap:hover .vtab-gear { color: var(--text-dim); }
  .vtab-wrap.active .vtab-gear { color: var(--text-dim); }
  .vtab-gear:hover { color: var(--accent) !important; }

  .vtab-add {
    padding: 5px 12px;
    font-size: 14px;
    color: var(--text-dim);
  }

  .vtab-rename-input {
    background: var(--input-bg);
    border: 1px solid var(--accent);
    color: var(--text);
    font-size: 12px;
    padding: 2px 6px;
    outline: none;
    width: 100px;
    border-radius: 3px;
    margin: 2px 2px;
  }

  .tab-ctx-backdrop {
    position: fixed;
    inset: 0;
    z-index: 8999;
  }
  .tab-ctx-menu {
    position: fixed;
    z-index: 9000;
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.4);
    padding: 4px 0;
    min-width: 140px;
    font-size: 12px;
  }
  .tab-ctx-item {
    display: block;
    width: 100%;
    background: none;
    border: none;
    color: var(--text);
    padding: 5px 12px;
    text-align: left;
    cursor: pointer;
    font-size: 12px;
  }
  .tab-ctx-item:hover { background: var(--hover); }
  .tab-ctx-item.danger { color: var(--red); }
  .tab-ctx-sep { height: 1px; background: var(--border); margin: 3px 0; }

  .search-input {
    background: var(--input-bg);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 12px;
    width: 150px;
    outline: none;
    -webkit-app-region: no-drag;
  }
  .search-input:focus { border-color: var(--accent); }

  .spacer { flex: 1; }

  .titlebar-btn {
    background: none;
    border: none;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 12px;
    padding: 4px 8px;
    border-radius: 4px;
    -webkit-app-region: no-drag;
  }
  .titlebar-btn:hover { color: var(--text); background: var(--hover); }

  .main-area {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--bg);
    min-width: 0;
  }

  .loading {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-dim);
    font-size: 13px;
  }
</style>
