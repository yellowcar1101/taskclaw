<script lang="ts">
  import { onMount } from 'svelte';
  import '../app.css';
  import TaskTree from '$lib/components/TaskTree.svelte';
  import GroupedView from '$lib/components/GroupedView.svelte';
  import ViewsPanel from '$lib/components/ViewsPanel.svelte';
  import TaskDetail from '$lib/components/TaskDetail.svelte';
  import SyncBar from '$lib/components/SyncBar.svelte';
  import Prefs from '$lib/components/Prefs.svelte';
  import ReminderWindow from '$lib/components/ReminderWindow.svelte';
  import RapidInput from '$lib/components/RapidInput.svelte';
  import {
    loadAll, views, activeTabId, detailTaskId, showPrefs, showRapidInput
  } from '$lib/stores/tasks';
  import type { SavedView } from '$lib/types';

  let ready = false;
  let reminderWindow: ReminderWindow;

  $: activeView = $views.find((v: SavedView) => v.id === $activeTabId) ?? null;

  onMount(async () => {
    await loadAll();
    ready = true;
    if (reminderWindow) reminderWindow.refresh();
    const interval = setInterval(() => { if (reminderWindow) reminderWindow.refresh(); }, 30000);
    return () => clearInterval(interval);
  });
</script>

<div class="app-shell">
  <!-- Titlebar -->
  <header class="titlebar">
    <span class="app-name">TaskClaw</span>
    <div class="tab-bar">
      <button
        class="tab"
        class:active={$activeTabId === 'outline'}
        on:click={() => activeTabId.set('outline')}
      >
        ☰ Outline
      </button>
      {#each $views as view (view.id)}
        <button
          class="tab"
          class:active={$activeTabId === view.id}
          on:click={() => activeTabId.set(view.id)}
        >
          {view.name}
        </button>
      {/each}
    </div>
    <div class="titlebar-right">
      <button class="tb-icon" on:click={() => showPrefs.set(true)} title="Preferences">⚙</button>
      <SyncBar />
    </div>
  </header>

  <!-- Main area -->
  <div class="main-area">
    <div class="content">
      {#if !ready}
        <div class="loading">Loading…</div>
      {:else if $activeTabId === 'outline'}
        <TaskTree />
      {:else if activeView}
        <GroupedView view={activeView} />
      {:else}
        <div class="loading">Select a view</div>
      {/if}
    </div>

    {#if $detailTaskId}
      <TaskDetail />
    {/if}

    <ViewsPanel />
  </div>
</div>

<!-- Overlays -->
<Prefs bind:open={$showPrefs} />
<ReminderWindow bind:this={reminderWindow} />
<RapidInput />

<svelte:window on:keydown={e => {
  if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'I') {
    e.preventDefault();
    showRapidInput.set(true);
  }
}} />

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
    padding: 0 0 0 12px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    gap: 8px;
    -webkit-app-region: drag;
    user-select: none;
  }

  .app-name {
    font-size: 12px;
    font-weight: 600;
    color: var(--accent);
    letter-spacing: 0.05em;
    font-family: 'Cascadia Code', 'Fira Code', monospace;
    -webkit-app-region: no-drag;
    flex-shrink: 0;
  }

  .tab-bar {
    display: flex;
    align-items: stretch;
    gap: 2px;
    flex: 1;
    overflow-x: auto;
    -webkit-app-region: no-drag;
    height: 100%;
    padding: 4px 0 0;
  }
  .tab-bar::-webkit-scrollbar { display: none; }

  .tab {
    background: none;
    border: none;
    border-top: 2px solid transparent;
    border-radius: 0;
    color: var(--text-dim);
    padding: 0 14px;
    cursor: pointer;
    font-size: 12px;
    font-family: sans-serif;
    white-space: nowrap;
    transition: color 0.1s, background 0.1s;
    align-self: stretch;
    display: flex;
    align-items: center;
  }
  .tab:hover { color: var(--text); background: var(--hover); }
  .tab.active {
    color: var(--accent);
    border-top-color: var(--accent);
    background: var(--bg);
  }

  .titlebar-right {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 8px;
    -webkit-app-region: no-drag;
    flex-shrink: 0;
  }

  .tb-icon {
    background: none;
    border: none;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 14px;
    padding: 3px 6px;
    border-radius: 4px;
    transition: color 0.1s, background 0.1s;
  }
  .tb-icon:hover { color: var(--text); background: var(--hover); }

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
  }

  .loading {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-dim);
    font-size: 13px;
    font-family: sans-serif;
  }
</style>
