<script lang="ts">
  import { onMount } from 'svelte';
  import '../app.css';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import TaskTree from '$lib/components/TaskTree.svelte';
  import { loadAll } from '$lib/stores/tasks';

  let ready = false;

  onMount(async () => {
    await loadAll();
    ready = true;
  });
</script>

<div class="app-shell">
  <header class="titlebar">
    <span class="app-name">TaskClaw</span>
    <span class="spacer"></span>
  </header>

  <div class="main-area">
    <Sidebar />
    <div class="content">
      {#if ready}
        <TaskTree />
      {:else}
        <div class="loading">Loading…</div>
      {/if}
    </div>
  </div>
</div>

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
    padding: 0 12px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
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
  }

  .spacer { flex: 1; }

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
