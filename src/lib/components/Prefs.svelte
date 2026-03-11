<script lang="ts">
  import { onMount } from 'svelte';
  import { showPrefs, flags, tags } from '../stores/tasks';
  import { api } from '../api';
  import type { Flag, Tag } from '../types';

  let activeTab: 'flags' | 'tags' | 'app' | 'sync' | 'api' = 'flags';
  let error = '';

  // ── App appearance settings ──────────────────────────────────────────────────
  const FONT_KEY    = 'app_font';
  const SIZE_KEY    = 'app_font_size';
  const COMPACT_KEY = 'app_compact';

  let appFont = localStorage.getItem(FONT_KEY) ?? 'system';
  let appFontSize = localStorage.getItem(SIZE_KEY) ?? '12';
  let appCompact = localStorage.getItem(COMPACT_KEY) === 'true';

  function applyAppearance() {
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
    root.style.setProperty('--app-font', fontMap[appFont] ?? fontMap.system);
    root.style.setProperty('--app-font-size', appFontSize + 'px');
    root.style.setProperty('--row-height', appCompact ? '22px' : '28px');
    localStorage.setItem(FONT_KEY, appFont);
    localStorage.setItem(SIZE_KEY, appFontSize);
    localStorage.setItem(COMPACT_KEY, String(appCompact));
  }

  // Apply on mount (load saved)
  onMount(() => {
    applyAppearance();
  });

  $: { appFont; appFontSize; appCompact; applyAppearance(); }

  // ── GDrive Sync ──────────────────────────────────────────────────────────────
  let gdriveConnected = false;
  let gdriveLastSync: string | null = null;
  let syncStatus = '';
  let syncing = false;
  let connecting = false;

  async function loadSyncStatus() {
    try {
      gdriveConnected = await api.gdriveStatus();
      gdriveLastSync = await api.gdriveLastSync();
    } catch {}
  }

  onMount(loadSyncStatus);

  async function connectGDrive() {
    connecting = true;
    syncStatus = '';
    try {
      const { url, port } = await api.gdriveAuthUrl();
      // Open browser
      const { openUrl } = await import('@tauri-apps/plugin-opener');
      await openUrl(url);
      syncStatus = 'Waiting for Google authorization…';
      // Wait for redirect (blocking call on Rust side)
      const msg = await api.gdriveWaitAuth(port);
      syncStatus = msg;
      await loadSyncStatus();
    } catch (e: any) {
      syncStatus = 'Error: ' + (e?.message ?? String(e));
    } finally {
      connecting = false;
    }
  }

  async function disconnect() {
    await api.gdriveDisconnect();
    gdriveConnected = false;
    gdriveLastSync = null;
    syncStatus = 'Disconnected.';
  }

  async function push() {
    syncing = true;
    syncStatus = '';
    try {
      syncStatus = await api.gdriveSyncPush();
      gdriveLastSync = await api.gdriveLastSync();
    } catch (e: any) {
      syncStatus = 'Error: ' + (e?.message ?? String(e));
    } finally {
      syncing = false;
    }
  }

  async function pull() {
    if (!confirm('This will REPLACE all local data with the cloud version. Continue?')) return;
    syncing = true;
    syncStatus = '';
    try {
      syncStatus = await api.gdriveSyncPull();
      // Reload all data
      const { loadAll } = await import('../stores/tasks');
      await loadAll();
    } catch (e: any) {
      syncStatus = 'Error: ' + (e?.message ?? String(e));
    } finally {
      syncing = false;
    }
  }

  // ── Flags ────────────────────────────────────────────────────────────────────
  let editingFlagId: string | null = null;
  let editFlagName = '';
  let editFlagColor = '#4A9EFF';
  let creatingFlag = false;
  let newFlagName = '';
  let newFlagColor = '#4A9EFF';

  function startEditFlag(f: Flag) {
    editingFlagId = f.id;
    editFlagName = f.name;
    editFlagColor = f.color;
  }

  async function commitFlag(f: Flag) {
    if (!editFlagName.trim()) { editingFlagId = null; return; }
    try {
      const updated = await api.updateFlag(f.id, editFlagName.trim(), editFlagColor);
      flags.update(fs => fs.map(x => x.id === f.id ? updated : x));
    } catch (e: any) { error = e?.message ?? String(e); }
    editingFlagId = null;
  }

  async function deleteFlag(id: string) {
    if (!confirm('Delete this flag?')) return;
    try {
      await api.deleteFlag(id);
      flags.update(fs => fs.filter(f => f.id !== id));
    } catch (e: any) { error = e?.message ?? String(e); }
  }

  async function createFlag() {
    if (!newFlagName.trim()) return;
    try {
      const f = await api.createFlag(newFlagName.trim(), newFlagColor);
      flags.update(fs => [...fs, f]);
      newFlagName = '';
      newFlagColor = '#4A9EFF';
      creatingFlag = false;
    } catch (e: any) { error = e?.message ?? String(e); }
  }

  // ── Tags ─────────────────────────────────────────────────────────────────────
  let editingTagId: string | null = null;
  let editTagName = '';
  let editTagColor = '#4A9EFF';
  let creatingTag = false;
  let newTagName = '';
  let newTagColor = '#6ABF69';

  function startEditTag(t: Tag) {
    editingTagId = t.id;
    editTagName = t.name;
    editTagColor = t.color;
  }

  async function commitTag(t: Tag) {
    if (!editTagName.trim()) { editingTagId = null; return; }
    try {
      const updated = await api.updateTag(t.id, editTagName.trim(), editTagColor);
      tags.update(ts => ts.map(x => x.id === t.id ? updated : x));
    } catch (e: any) { error = e?.message ?? String(e); }
    editingTagId = null;
  }

  async function deleteTag(id: string) {
    if (!confirm('Delete this tag?')) return;
    try {
      await api.deleteTag(id);
      tags.update(ts => ts.filter(t => t.id !== id));
    } catch (e: any) { error = e?.message ?? String(e); }
  }

  async function createTag() {
    if (!newTagName.trim()) return;
    try {
      const t = await api.createTag(newTagName.trim(), newTagColor);
      tags.update(ts => [...ts, t]);
      newTagName = '';
      newTagColor = '#6ABF69';
      creatingTag = false;
    } catch (e: any) { error = e?.message ?? String(e); }
  }

  // ── Web API ───────────────────────────────────────────────────────────────
  let apiPort = 7380;
  let apiStatus: { port: string | null; has_token: boolean } | null = null;
  let apiToken = '';
  let apiMsg = '';
  let apiStarting = false;

  async function loadApiStatus() {
    try { apiStatus = await api.webapiStatus(); } catch {}
  }

  async function startApi() {
    apiStarting = true;
    apiMsg = '';
    try {
      const msg = await api.webapiStart(apiPort);
      apiMsg = msg;
      await loadApiStatus();
    } catch (e: any) {
      apiMsg = 'Error: ' + (e?.message ?? String(e));
    } finally {
      apiStarting = false;
    }
  }

  async function saveToken() {
    if (!apiToken.trim()) return;
    try {
      await api.webapiSetToken(apiToken.trim());
      apiMsg = 'Token saved. Restart API to apply.';
      apiToken = '';
      await loadApiStatus();
    } catch (e: any) {
      apiMsg = 'Error: ' + (e?.message ?? String(e));
    }
  }

  function genToken() {
    const arr = new Uint8Array(24);
    crypto.getRandomValues(arr);
    apiToken = btoa(String.fromCharCode(...arr)).replace(/[+/=]/g, '').slice(0, 32);
  }

  onMount(loadApiStatus);

  function close() { showPrefs.set(false); }
  function onKeydown(e: KeyboardEvent) { if (e.key === 'Escape') close(); }
</script>

<svelte:window on:keydown={onKeydown} />

<div class="overlay" on:click|self={close} role="dialog" aria-modal="true">
  <div class="modal" role="document">
    <!-- Header -->
    <div class="modal-header">
      <span class="modal-title">⚙ Preferences</span>
      <button class="close-btn" on:click={close}>✕</button>
    </div>

    <!-- Tabs -->
    <div class="tabs">
      <button class="tab" class:active={activeTab === 'flags'} on:click={() => activeTab = 'flags'}>Flags</button>
      <button class="tab" class:active={activeTab === 'tags'}  on:click={() => activeTab = 'tags'}>Tags</button>
      <button class="tab" class:active={activeTab === 'app'}   on:click={() => activeTab = 'app'}>Appearance</button>
      <button class="tab" class:active={activeTab === 'sync'}  on:click={() => activeTab = 'sync'}>Sync</button>
      <button class="tab" class:active={activeTab === 'api'}   on:click={() => activeTab = 'api'}>API</button>
    </div>

    {#if error}
      <div class="err-bar">{error} <button on:click={() => error = ''}>✕</button></div>
    {/if}

    <!-- Body -->
    <div class="modal-body">

      <!-- FLAGS TAB -->
      {#if activeTab === 'flags'}
        <div class="section-hint">Flags are priority/status markers shown on tasks. Up to 8 recommended.</div>
        {#each $flags as flag (flag.id)}
          <div class="item-row">
            <span class="color-dot" style="background:{flag.color}"></span>
            {#if editingFlagId === flag.id}
              <input class="name-input" bind:value={editFlagName}
                on:blur={() => commitFlag(flag)}
                on:keydown={e => { if (e.key === 'Enter') commitFlag(flag); if (e.key === 'Escape') editingFlagId = null; }}
                autofocus />
              <input type="color" class="color-picker" bind:value={editFlagColor} />
              <button class="icon-btn accent" on:click={() => commitFlag(flag)}>✓</button>
            {:else}
              <span class="item-name">{flag.name}</span>
              <button class="icon-btn" on:click={() => startEditFlag(flag)}>✎</button>
              <button class="icon-btn danger" on:click={() => deleteFlag(flag.id)}>✕</button>
            {/if}
          </div>
        {/each}
        {#if creatingFlag}
          <div class="item-row">
            <input type="color" class="color-picker" bind:value={newFlagColor} />
            <input class="name-input" bind:value={newFlagName} placeholder="Flag name…"
              on:keydown={e => { if (e.key === 'Enter') createFlag(); if (e.key === 'Escape') { creatingFlag = false; newFlagName = ''; } }}
              autofocus />
            <button class="icon-btn accent" on:click={createFlag}>✓</button>
            <button class="icon-btn" on:click={() => { creatingFlag = false; newFlagName = ''; }}>✕</button>
          </div>
        {:else}
          <button class="add-btn" on:click={() => creatingFlag = true}>+ Add Flag</button>
        {/if}
      {/if}

      <!-- TAGS TAB -->
      {#if activeTab === 'tags'}
        <div class="section-hint">Tags are free-form labels for filtering and grouping tasks.</div>
        {#each $tags as tag (tag.id)}
          <div class="item-row">
            <span class="color-dot" style="background:{tag.color}"></span>
            {#if editingTagId === tag.id}
              <input class="name-input" bind:value={editTagName}
                on:blur={() => commitTag(tag)}
                on:keydown={e => { if (e.key === 'Enter') commitTag(tag); if (e.key === 'Escape') editingTagId = null; }}
                autofocus />
              <input type="color" class="color-picker" bind:value={editTagColor} />
              <button class="icon-btn accent" on:click={() => commitTag(tag)}>✓</button>
            {:else}
              <span class="item-name">{tag.name}</span>
              <button class="icon-btn" on:click={() => startEditTag(tag)}>✎</button>
              <button class="icon-btn danger" on:click={() => deleteTag(tag.id)}>✕</button>
            {/if}
          </div>
        {/each}
        {#if creatingTag}
          <div class="item-row">
            <input type="color" class="color-picker" bind:value={newTagColor} />
            <input class="name-input" bind:value={newTagName} placeholder="Tag name…"
              on:keydown={e => { if (e.key === 'Enter') createTag(); if (e.key === 'Escape') { creatingTag = false; newTagName = ''; } }}
              autofocus />
            <button class="icon-btn accent" on:click={createTag}>✓</button>
            <button class="icon-btn" on:click={() => { creatingTag = false; newTagName = ''; }}>✕</button>
          </div>
        {:else}
          <button class="add-btn" on:click={() => creatingTag = true}>+ Add Tag</button>
        {/if}
      {/if}

      <!-- APPEARANCE TAB -->
      {#if activeTab === 'app'}
        <div class="section-hint">Appearance settings apply immediately.</div>

        <div class="info-row">
          <span class="info-label">Font</span>
          <select class="info-select" bind:value={appFont}>
            <optgroup label="Sans-serif (recommended)">
              <option value="system">System UI (default)</option>
              <option value="segoe">Segoe UI</option>
              <option value="inter">Inter</option>
              <option value="verdana">Verdana</option>
              <option value="trebuchet">Trebuchet MS</option>
              <option value="calibri">Calibri</option>
              <option value="roboto">Roboto</option>
              <option value="opensans">Open Sans</option>
            </optgroup>
            <optgroup label="Serif">
              <option value="georgia">Georgia</option>
              <option value="garamond">Garamond</option>
              <option value="palatino">Palatino Linotype</option>
              <option value="times">Times New Roman</option>
            </optgroup>
            <optgroup label="Monospace">
              <option value="mono">Cascadia Code / Fira Code</option>
              <option value="consolas">Consolas</option>
              <option value="courier">Courier New</option>
            </optgroup>
          </select>
        </div>

        <div class="info-row">
          <span class="info-label">Font size</span>
          <div style="display:flex;align-items:center;gap:8px">
            <input type="range" min="10" max="16" step="1" bind:value={appFontSize} style="width:100px" />
            <span class="info-value">{appFontSize}px</span>
          </div>
        </div>

        <div class="info-row">
          <span class="info-label">Row height</span>
          <label style="display:flex;align-items:center;gap:8px;font-size:12px;cursor:pointer">
            <input type="checkbox" bind:checked={appCompact} style="accent-color:var(--accent)" />
            Compact mode (22px rows)
          </label>
        </div>

        <div class="info-row" style="margin-top:16px">
          <span class="info-label">Version</span>
          <span class="info-value">TaskClaw 0.3.0</span>
        </div>
        <div class="info-row">
          <span class="info-label">Data</span>
          <span class="info-value dim">./Data/tasks.db (portable)</span>
        </div>
      {/if}

      <!-- SYNC TAB -->
      {#if activeTab === 'sync'}
        <div class="section-hint">
          Sync your tasks to Google Drive. The sync file (<code>taskclaw-sync.json</code>) is stored
          in your Drive root. Last-write-wins — push overwrites cloud, pull overwrites local.
        </div>

        {#if gdriveConnected}
          <div class="sync-status connected">
            <span class="sync-dot connected"></span>
            Connected to Google Drive
          </div>
          {#if gdriveLastSync}
            <div class="info-row">
              <span class="info-label">Last sync</span>
              <span class="info-value dim">{new Date(gdriveLastSync).toLocaleString()}</span>
            </div>
          {/if}
          <div class="sync-actions">
            <button class="sync-btn push" on:click={push} disabled={syncing}>
              {syncing ? '…' : '↑ Push to Cloud'}
            </button>
            <button class="sync-btn pull" on:click={pull} disabled={syncing}>
              {syncing ? '…' : '↓ Pull from Cloud'}
            </button>
            <button class="sync-btn danger" on:click={disconnect}>Disconnect</button>
          </div>
        {:else}
          <div class="sync-status">
            <span class="sync-dot"></span>
            Not connected
          </div>
          <button class="sync-btn connect" on:click={connectGDrive} disabled={connecting}>
            {connecting ? 'Connecting…' : '🔗 Connect to Google Drive'}
          </button>
          <div class="section-hint" style="margin-top:8px">
            Your browser will open to authorize TaskClaw. After approval, return here.
          </div>
        {/if}

        {#if syncStatus}
          <div class="sync-msg" class:err={syncStatus.startsWith('Error')}>{syncStatus}</div>
        {/if}
      {/if}

      <!-- API TAB -->
      {#if activeTab === 'api'}
        <div class="section-hint">
          REST API lets external apps read your tasks over HTTP.
          Default port: <code>7380</code> — access at <code>http://localhost:7380/api/tasks</code>.
          The API persists across restarts once started.
        </div>

        {#if apiStatus?.port}
          <div class="sync-status connected">
            <span class="sync-dot connected"></span>
            Running on port {apiStatus.port}
            {#if apiStatus.has_token}· token protected{/if}
          </div>
        {:else}
          <div class="sync-status">
            <span class="sync-dot"></span>
            Not running
          </div>
        {/if}

        <div class="info-row">
          <span class="info-label">Port</span>
          <input
            type="number" min="1024" max="65535"
            class="name-input" style="max-width:90px"
            bind:value={apiPort}
          />
        </div>

        <div style="margin: 10px 0">
          <button class="sync-btn push" on:click={startApi} disabled={apiStarting}>
            {apiStarting ? '…' : apiStatus?.port ? '↺ Restart API' : '▶ Start API'}
          </button>
        </div>

        <div class="info-row" style="margin-top:12px">
          <span class="info-label">Auth Token</span>
          <div style="display:flex;gap:6px;flex:1">
            <input class="name-input" placeholder="Paste or generate…" bind:value={apiToken} style="flex:1" />
            <button class="icon-btn" on:click={genToken} title="Generate random token">⚡</button>
            <button class="icon-btn accent" on:click={saveToken} title="Save token">✓</button>
          </div>
        </div>
        <div class="section-hint" style="margin-top:4px">
          {#if apiStatus?.has_token}
            A token is set. Send <code>Authorization: Bearer &lt;token&gt;</code> in requests.
            To change, enter a new token above.
          {:else}
            No token set — API is open to all local network clients.
          {/if}
        </div>

        {#if apiMsg}
          <div class="sync-msg" class:err={apiMsg.startsWith('Error')}>{apiMsg}</div>
        {/if}

        <div class="section-hint" style="margin-top:16px">
          <strong>Endpoints:</strong><br>
          GET /api/tasks · GET /api/tasks/:id<br>
          GET /api/flags · GET /api/tags · GET /api/views<br>
          GET /api/health
        </div>
      {/if}

    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.5);
    z-index: 800;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .modal {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    width: min(520px, 92vw);
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 16px 48px rgba(0,0,0,0.5);
  }

  .modal-header {
    display: flex;
    align-items: center;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .modal-title { font-size: 14px; font-weight: 600; flex: 1; }
  .close-btn { background: none; border: none; color: var(--text-dim); cursor: pointer; font-size: 14px; }
  .close-btn:hover { color: var(--text); }

  .tabs {
    display: flex;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    padding: 0 12px;
  }
  .tab {
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--text-dim);
    padding: 7px 14px;
    cursor: pointer;
    font-size: 12px;
    transition: color 0.1s;
    margin-bottom: -1px;
  }
  .tab:hover { color: var(--text); }
  .tab.active { color: var(--accent); border-bottom-color: var(--accent); }

  .err-bar {
    background: #E05C5C22;
    border-bottom: 1px solid var(--red);
    color: var(--red);
    font-size: 11px;
    padding: 6px 14px;
    display: flex;
    justify-content: space-between;
    flex-shrink: 0;
  }
  .err-bar button { background: none; border: none; color: var(--red); cursor: pointer; }

  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 12px 16px;
  }

  .section-hint {
    font-size: 11px;
    color: var(--text-dim);
    margin-bottom: 10px;
    line-height: 1.5;
  }

  .item-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 4px;
    border-radius: 4px;
  }
  .item-row:hover { background: var(--hover); }

  .color-dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .color-picker {
    width: 28px;
    height: 22px;
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
    padding: 1px;
    background: var(--input-bg);
    flex-shrink: 0;
  }

  .item-name {
    flex: 1;
    font-size: 13px;
    color: var(--text);
  }

  .name-input {
    flex: 1;
    background: var(--input-bg);
    border: 1px solid var(--accent);
    color: var(--text);
    padding: 3px 8px;
    border-radius: 3px;
    font-size: 12px;
    outline: none;
  }

  .icon-btn {
    background: none;
    border: none;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 12px;
    padding: 2px 6px;
    border-radius: 3px;
  }
  .icon-btn:hover { color: var(--text); background: var(--hover-btn); }
  .icon-btn.danger:hover { color: var(--red); }
  .icon-btn.accent { color: var(--accent); }

  .add-btn {
    margin-top: 6px;
    background: var(--hover-btn);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 4px 12px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
  }
  .add-btn:hover { background: var(--hover); }

  .info-row {
    display: flex;
    align-items: center;
    padding: 6px 4px;
    border-bottom: 1px solid var(--border);
    font-size: 12px;
  }
  .info-label { flex: 1; color: var(--text-dim); min-width: 100px; }
  .info-value { color: var(--text); }
  .info-value.dim { color: var(--text-dim); font-style: italic; }
  .info-select {
    background: var(--input-bg);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 3px 6px;
    border-radius: 4px;
    font-size: 12px;
    outline: none;
    flex: 1;
  }

  .sync-status {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--text-dim);
    padding: 8px 4px;
    margin-bottom: 8px;
  }
  .sync-dot {
    width: 8px; height: 8px;
    border-radius: 50%;
    background: var(--border);
    flex-shrink: 0;
  }
  .sync-dot.connected { background: var(--green); }
  .sync-status.connected { color: var(--green); }

  .sync-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin: 12px 0;
  }
  .sync-btn {
    padding: 6px 14px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    border: 1px solid var(--border);
    background: var(--hover-btn);
    color: var(--text);
  }
  .sync-btn:hover:not(:disabled) { background: var(--hover); }
  .sync-btn:disabled { opacity: 0.5; cursor: default; }
  .sync-btn.push { background: var(--accent); border-color: var(--accent); color: #fff; }
  .sync-btn.pull { background: var(--accent-dim); border-color: var(--accent); color: var(--accent); }
  .sync-btn.connect { background: var(--accent); border-color: var(--accent); color: #fff; font-size: 13px; padding: 8px 18px; }
  .sync-btn.danger { color: var(--red); border-color: var(--red); }
  .sync-msg {
    font-size: 11px;
    color: var(--green);
    padding: 6px 4px;
    border-radius: 3px;
    background: #6ABF6922;
  }
  .sync-msg.err { color: var(--red); background: #E05C5C22; }
</style>
