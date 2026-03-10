<script lang="ts">
  import { onMount } from 'svelte';
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
    filterFlagId, flags, searchQuery
  } from '$lib/stores/tasks';

  let ready = false;

  onMount(async () => {
    // Apply stored appearance before data loads
    const root = document.documentElement;
    const fontMap: Record<string, string> = {
      system: 'system-ui, sans-serif',
      mono:   "'Cascadia Code', 'Fira Code', monospace",
      inter:  'Inter, system-ui, sans-serif',
    };
    const savedFont = localStorage.getItem('app_font') ?? 'system';
    const savedSize = localStorage.getItem('app_font_size') ?? '12';
    const savedCompact = localStorage.getItem('app_compact') === 'true';
    root.style.setProperty('--app-font', fontMap[savedFont] ?? fontMap.system);
    root.style.setProperty('--app-font-size', savedSize + 'px');
    root.style.setProperty('--row-height', savedCompact ? '22px' : '28px');

    await loadAll();
    ready = true;
  });

  // Current view (null = outline)
  $: currentView = $activeTabId === 'outline'
    ? null
    : $views.find(v => v.id === $activeTabId) ?? null;

  // Views dropdown
  let showViewDropdown = false;

  function selectView(id: string) {
    activeTabId.set(id);
    showViewDropdown = false;
  }

  function onGlobalKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') showViewDropdown = false;
  }
</script>

<svelte:window on:keydown={onGlobalKeydown} />

<div class="app-shell">
  <!-- Titlebar -->
  <header class="titlebar">
    <span class="app-name">TaskClaw</span>

    <!-- View selector -->
    {#if ready}
      <div class="view-selector" style="position:relative">
        <button
          class="view-btn"
          class:active={showViewDropdown}
          on:click={() => showViewDropdown = !showViewDropdown}
        >
          {currentView ? currentView.name : 'Outline'} ▾
        </button>
        {#if showViewDropdown}
          <div class="view-dropdown" role="listbox">
            <!-- Built-in -->
            <div
              class="view-opt"
              class:selected={$activeTabId === 'outline'}
              on:click={() => selectView('outline')}
              role="option" tabindex="0" on:keydown
            >☰ Outline</div>
            <div
              class="view-opt"
              class:selected={$activeTabId === '__starred__'}
              on:click={() => selectView('__starred__')}
              role="option" tabindex="0" on:keydown
            >⭐ Starred</div>
            <div
              class="view-opt"
              class:selected={$activeTabId === '__today__'}
              on:click={() => selectView('__today__')}
              role="option" tabindex="0" on:keydown
            >◷ Due Today</div>

            {#if $views.length}
              <div class="view-sep"></div>
              {#each $views as view (view.id)}
                <div class="view-opt-row">
                  <div
                    class="view-opt flex1"
                    class:selected={$activeTabId === view.id}
                    on:click={() => selectView(view.id)}
                    role="option" tabindex="0" on:keydown
                  >{view.name}</div>
                  <button
                    class="view-gear"
                    on:click|stopPropagation={() => { editingViewId.set(view.id); showViewDropdown = false; }}
                    title="View settings"
                  >⚙</button>
                </div>
              {/each}
            {/if}

            <div class="view-sep"></div>
            <div
              class="view-opt new-view"
              on:click={async () => {
                showViewDropdown = false;
                const { api } = await import('$lib/api');
                const v = await api.createView({
                  name: 'New View', show_completed: false, group_by: 'none',
                  sort_by: 'position', sort_dir: 'asc', visible_fields: [],
                  filter_json: '{"action_filter":"active"}'
                });
                const { views: vs } = await import('$lib/stores/tasks');
                vs.update(arr => [...arr, v]);
                editingViewId.set(v.id);
              }}
              role="option" tabindex="0" on:keydown
            >+ New View…</div>
          </div>
        {/if}
      </div>

      <!-- Flag filter (moved from sidebar) -->
      <select class="flag-filter" bind:value={$filterFlagId}>
        <option value={null}>All flags</option>
        <option value="__starred__">⭐ Starred</option>
        <option value="__today__">◷ Due Today</option>
        {#each $flags as flag}
          <option value={flag.id}>{flag.name}</option>
        {/each}
      </select>

      <!-- Search -->
      <input
        class="search-input"
        placeholder="Search…"
        bind:value={$searchQuery}
      />
    {/if}

    <div class="spacer"></div>
    <button class="titlebar-btn" on:click={() => showPrefs.set(true)} title="Preferences">⚙ Prefs</button>
  </header>

  <!-- Main area -->
  <div class="main-area">
    <div class="content">
      {#if !ready}
        <div class="loading">Loading…</div>
      {:else if $activeTabId === 'outline'}
        <TaskTree />
      {:else if $activeTabId === '__starred__' || $activeTabId === '__today__'}
        <!-- Built-in flat views -->
        {#each [{ id: $activeTabId, name: $activeTabId === '__starred__' ? 'Starred' : 'Due Today',
                  show_completed: false, group_by: 'none', sort_by: 'due_date', sort_dir: 'asc',
                  visible_fields: [], filter_json: '{"action_filter":"all"}', position: 0 }] as v}
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

    <!-- Task Detail panel -->
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
    padding: 0 8px 0 12px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    -webkit-app-region: drag;
    user-select: none;
    gap: 6px;
  }

  .app-name {
    font-size: 12px;
    font-weight: 600;
    color: var(--accent);
    letter-spacing: 0.05em;
    font-family: 'Cascadia Code', monospace;
    -webkit-app-region: no-drag;
    flex-shrink: 0;
  }

  .view-selector { -webkit-app-region: no-drag; }

  .view-btn {
    background: var(--hover-btn);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 3px 10px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    transition: border-color 0.1s;
    white-space: nowrap;
  }
  .view-btn:hover, .view-btn.active { border-color: var(--accent); }

  .view-dropdown {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    min-width: 200px;
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.4);
    z-index: 900;
    padding: 4px 0;
  }

  .view-opt {
    padding: 6px 12px;
    font-size: 12px;
    color: var(--text-dim);
    cursor: pointer;
    border-radius: 3px;
    margin: 0 4px;
  }
  .view-opt:hover { background: var(--hover); color: var(--text); }
  .view-opt.selected { color: var(--accent); }
  .view-opt.new-view { color: var(--accent); font-weight: 500; }
  .view-opt.flex1 { flex: 1; }

  .view-opt-row {
    display: flex;
    align-items: center;
    margin: 0 4px;
    border-radius: 3px;
  }
  .view-opt-row:hover { background: var(--hover); }
  .view-opt-row .view-opt { margin: 0; border-radius: 0; }

  .view-gear {
    background: none;
    border: none;
    color: var(--text-dim);
    cursor: pointer;
    padding: 0 8px;
    font-size: 11px;
    flex-shrink: 0;
  }
  .view-gear:hover { color: var(--accent); }

  .view-sep { height: 1px; background: var(--border); margin: 3px 0; }

  .flag-filter {
    background: var(--input-bg);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 3px 6px;
    border-radius: 4px;
    font-size: 12px;
    outline: none;
    -webkit-app-region: no-drag;
    max-width: 130px;
  }

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
