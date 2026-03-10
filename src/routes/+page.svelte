<script lang="ts">
  import { onMount } from 'svelte';
  import '../app.css';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import TaskTree from '$lib/components/TaskTree.svelte';
  import TaskDetail from '$lib/components/TaskDetail.svelte';
  import ContextMenu from '$lib/components/ContextMenu.svelte';
  import RapidInput from '$lib/components/RapidInput.svelte';
  import ReminderWindow from '$lib/components/ReminderWindow.svelte';
  import ViewsPanel from '$lib/components/ViewsPanel.svelte';
  import Prefs from '$lib/components/Prefs.svelte';
  import {
    loadAll, detailTaskId, showPrefs, showRapidInput,
    showViewsPanel, contextMenu, views, activeTabId
  } from '$lib/stores/tasks';

  let ready = false;

  onMount(async () => {
    await loadAll();
    ready = true;
  });
</script>

<div class="app-shell">
  <!-- Titlebar -->
  <header class="titlebar">
    <span class="app-name">TaskClaw</span>

    <!-- Tabs: Outline + saved views -->
    {#if ready}
      <div class="tab-bar">
        <button
          class="titlebar-tab"
          class:active={$activeTabId === 'outline'}
          on:click={() => activeTabId.set('outline')}
        >Outline</button>
        {#each $views as view (view.id)}
          <button
            class="titlebar-tab"
            class:active={$activeTabId === view.id}
            on:click={() => activeTabId.set(view.id)}
          >{view.name}</button>
        {/each}
      </div>
    {/if}

    <div class="spacer"></div>
    <button class="titlebar-btn" on:click={() => showViewsPanel.set(true)} title="Saved Views">⊞ Views</button>
    <button class="titlebar-btn" on:click={() => showPrefs.set(true)} title="Preferences">⚙</button>
  </header>

  <!-- Main area -->
  <div class="main-area">
    <Sidebar />

    <div class="content">
      {#if !ready}
        <div class="loading">Loading…</div>
      {:else}
        <TaskTree />
      {/if}
    </div>

    <!-- Task Detail panel -->
    {#if $detailTaskId}
      <TaskDetail />
    {/if}
  </div>
</div>

<!-- Portalled overlays -->
{#if $contextMenu}
  <ContextMenu />
{/if}

{#if $showRapidInput}
  <RapidInput />
{/if}

{#if $showPrefs}
  <Prefs />
{/if}

{#if $showViewsPanel}
  <ViewsPanel />
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
    height: 34px;
    padding: 0 8px 0 12px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    -webkit-app-region: drag;
    user-select: none;
    gap: 4px;
  }

  .app-name {
    font-size: 12px;
    font-weight: 600;
    color: var(--accent);
    letter-spacing: 0.05em;
    font-family: 'Cascadia Code', 'Fira Code', monospace;
    -webkit-app-region: no-drag;
    flex-shrink: 0;
    margin-right: 8px;
  }

  .tab-bar {
    display: flex;
    align-items: stretch;
    gap: 1px;
    height: 100%;
    -webkit-app-region: no-drag;
    overflow: hidden;
  }

  .titlebar-tab {
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--text-dim);
    padding: 0 14px;
    cursor: pointer;
    font-size: 12px;
    height: 100%;
    white-space: nowrap;
    transition: color 0.1s;
  }
  .titlebar-tab:hover { color: var(--text); }
  .titlebar-tab.active {
    color: var(--text);
    border-bottom-color: var(--accent);
  }

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
    transition: color 0.1s, background 0.1s;
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
