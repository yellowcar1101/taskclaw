<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../api';
  import { loadAll } from '../stores/tasks';

  let connected = false;
  let syncing = false;
  let lastSync = '';
  let msg = '';
  let msgType: 'ok' | 'err' | '' = '';

  onMount(async () => {
    connected = await api.gdriveAuthStatus();
  });

  function flash(text: string, type: 'ok' | 'err') {
    msg = text; msgType = type;
    setTimeout(() => { msg = ''; msgType = ''; }, 4000);
  }

  async function connect() {
    syncing = true;
    try {
      const r = await api.gdriveConnect();
      connected = r.success;
      flash(r.message, 'ok');
    } catch (e: any) {
      flash(e.toString(), 'err');
    }
    syncing = false;
  }

  async function upload() {
    syncing = true;
    try {
      const r = await api.gdriveUpload();
      if (r.synced_at) lastSync = new Date(r.synced_at).toLocaleTimeString();
      flash(r.message, 'ok');
    } catch (e: any) {
      flash(e.toString(), 'err');
    }
    syncing = false;
  }

  async function download() {
    syncing = true;
    try {
      const r = await api.gdriveDownload();
      if (r.success && r.synced_at) {
        await loadAll(); // reload task list with fresh data
        lastSync = new Date(r.synced_at).toLocaleTimeString();
      }
      flash(r.message, 'ok');
    } catch (e: any) {
      flash(e.toString(), 'err');
    }
    syncing = false;
  }
</script>

<div class="sync-bar">
  {#if !connected}
    <button class="sync-btn connect" on:click={connect} disabled={syncing}>
      ☁ Connect Drive
    </button>
  {:else}
    <span class="sync-status" title="Google Drive connected">☁</span>
    <button class="sync-btn" on:click={upload} disabled={syncing} title="Upload to Drive">↑</button>
    <button class="sync-btn" on:click={download} disabled={syncing} title="Download from Drive">↓</button>
    {#if lastSync}<span class="last-sync">{lastSync}</span>{/if}
  {/if}

  {#if syncing}<span class="sync-spinner">⟳</span>{/if}

  {#if msg}
    <span class="sync-msg" class:ok={msgType === 'ok'} class:err={msgType === 'err'}>{msg}</span>
  {/if}
</div>

<style>
  .sync-bar {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .sync-btn {
    background: var(--hover-btn);
    border: 1px solid var(--border);
    color: var(--text-dim);
    padding: 2px 8px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    transition: color 0.1s, background 0.1s;
  }
  .sync-btn:hover:not(:disabled) { color: var(--text); background: var(--hover); }
  .sync-btn:disabled { opacity: 0.4; cursor: default; }
  .sync-btn.connect { color: var(--accent); border-color: var(--accent-dim); }
  .sync-status { font-size: 14px; color: var(--accent); }
  .last-sync { font-size: 10px; color: var(--text-dim); font-family: sans-serif; }
  .sync-spinner { font-size: 13px; color: var(--accent); animation: spin 1s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  .sync-msg { font-size: 11px; font-family: sans-serif; max-width: 200px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .sync-msg.ok  { color: var(--green); }
  .sync-msg.err { color: var(--red); }
</style>
