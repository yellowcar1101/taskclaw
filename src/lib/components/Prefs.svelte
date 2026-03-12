<script lang="ts">
  import { onMount } from 'svelte';
  import { showPrefs, flags, tags, themeFormatting } from '../stores/tasks';
  import { api } from '../api';
  import ColorPicker from './ColorPicker.svelte';
  import type { Flag, Tag } from '../types';
  import type { TaskTypeFormat, FormatKey } from '../stores/tasks';

  let activeTab: 'general' | 'flags' | 'tags' | 'app' | 'sync' | 'api' | 'formatting' = 'general';

  // ── Theme Formatting ──────────────────────────────────────────────────────
  const formatSections: { key: FormatKey; label: string }[] = [
    { key: 'active',    label: 'Active tasks format' },
    { key: 'project',   label: 'Projects format' },
    { key: 'folder',    label: 'Folders format' },
    { key: 'completed', label: 'Completed tasks format' },
    { key: 'hidden',    label: 'Hidden in todo format' },
  ];

  function updateFmt(key: FormatKey, field: keyof TaskTypeFormat, value: unknown) {
    themeFormatting.update(tf => ({
      ...tf,
      [key]: { ...tf[key], [field]: value },
    }));
  }
  let error = '';

  // ── App appearance settings ──────────────────────────────────────────────────
  const FONT_KEY       = 'app_font';
  const SIZE_KEY       = 'app_font_size';
  const COMPACT_KEY    = 'app_compact';
  const TASK_COLOR_KEY = 'app_task_color';

  let appFont      = localStorage.getItem(FONT_KEY) ?? 'system';
  let appFontSize  = localStorage.getItem(SIZE_KEY) ?? '12';
  let appCompact   = localStorage.getItem(COMPACT_KEY) === 'true';
  let appTaskColor = localStorage.getItem(TASK_COLOR_KEY) ?? '';

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
    if (appTaskColor) root.style.setProperty('--task-color', appTaskColor);
    else root.style.removeProperty('--task-color');
    localStorage.setItem(FONT_KEY, appFont);
    localStorage.setItem(SIZE_KEY, appFontSize);
    localStorage.setItem(COMPACT_KEY, String(appCompact));
    localStorage.setItem(TASK_COLOR_KEY, appTaskColor);
  }

  onMount(() => { applyAppearance(); });

  $: { appFont; appFontSize; appCompact; appTaskColor; applyAppearance(); }

  // ── General / Startup settings ────────────────────────────────────────────
  let rememberPosition = false;
  let singleInstance = true;
  let generalMsg = '';

  async function loadGeneralSettings() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const cfg = await invoke<{ remember_position: boolean; single_instance: boolean }>('get_startup_config');
      rememberPosition = cfg.remember_position;
      singleInstance = cfg.single_instance;
    } catch {}
  }

  async function saveGeneralSettings() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_startup_config', { config: { remember_position: rememberPosition, single_instance: singleInstance } });
      // mirror remember_position to localStorage so +page.svelte can read it for the beforeunload handler
      localStorage.setItem('startup_remember_position', String(rememberPosition));
      generalMsg = 'Saved. Restart required for single-instance changes.';
      setTimeout(() => generalMsg = '', 4000);
    } catch (e: any) {
      generalMsg = 'Error: ' + (e?.message ?? String(e));
    }
  }

  onMount(loadGeneralSettings);

  // ── Folder Sync ───────────────────────────────────────────────────────────────
  let syncFolder: string | null = null;
  let folderLastSync: string | null = null;
  let syncing = false;
  let syncStatus = '';

  async function loadSyncStatus() {
    try {
      syncFolder = await api.getSyncFolder();
      folderLastSync = await api.folderLastSync();
    } catch {}
  }

  onMount(loadSyncStatus);

  async function chooseFolder() {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const result = await open({ directory: true, multiple: false, title: 'Choose sync folder' });
      if (result) {
        await api.setSyncFolder(result as string);
        syncFolder = result as string;
        syncStatus = '';
      }
    } catch (e: any) {
      syncStatus = 'Error: ' + (e?.message ?? String(e));
    }
  }

  async function push() {
    syncing = true; syncStatus = '';
    try {
      const ts = await api.folderSyncPush();
      folderLastSync = ts;
      syncStatus = 'ok';
    } catch (e: any) {
      syncStatus = 'Error: ' + (e?.message ?? String(e));
    } finally { syncing = false; }
  }

  async function pull() {
    if (!confirm('This will REPLACE all local data with the version in the sync file. Continue?')) return;
    syncing = true; syncStatus = '';
    try {
      await api.folderSyncPull();
      folderLastSync = await api.folderLastSync();
      const { loadAll } = await import('../stores/tasks');
      await loadAll();
      syncStatus = 'ok';
    } catch (e: any) {
      syncStatus = 'Error: ' + (e?.message ?? String(e));
    } finally { syncing = false; }
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
      <button class="tab" class:active={activeTab === 'general'} on:click={() => activeTab = 'general'}>General</button>
      <button class="tab" class:active={activeTab === 'flags'}   on:click={() => activeTab = 'flags'}>Flags</button>
      <button class="tab" class:active={activeTab === 'tags'}    on:click={() => activeTab = 'tags'}>Tags</button>
      <button class="tab" class:active={activeTab === 'app'}     on:click={() => activeTab = 'app'}>Appearance</button>
      <button class="tab" class:active={activeTab === 'sync'}    on:click={() => activeTab = 'sync'}>Sync</button>
      <button class="tab" class:active={activeTab === 'api'}        on:click={() => activeTab = 'api'}>API</button>
      <button class="tab" class:active={activeTab === 'formatting'} on:click={() => activeTab = 'formatting'}>Formatting</button>
    </div>

    {#if error}
      <div class="err-bar">{error} <button on:click={() => error = ''}>✕</button></div>
    {/if}

    <!-- Body -->
    <div class="modal-body">

      <!-- GENERAL TAB -->
      {#if activeTab === 'general'}
        <div class="section-hint">These settings take effect on next launch.</div>

        <div class="general-grid">
          <label class="general-option">
            <input type="checkbox" bind:checked={rememberPosition} style="accent-color:var(--accent)"
              on:change={saveGeneralSettings} />
            <span>
              <strong>Remember window position</strong><br>
              Save and restore size &amp; position on startup
            </span>
          </label>

          <label class="general-option">
            <input type="checkbox" bind:checked={singleInstance} style="accent-color:var(--accent)"
              on:change={saveGeneralSettings} />
            <span>
              <strong>Single instance</strong><br>
              Prevent more than one copy running at a time
            </span>
          </label>

          <div class="general-option info-only">
            <span class="info-label">Version</span>
            <span class="info-value">TaskClaw 0.3.0</span>
          </div>

          <div class="general-option info-only">
            <span class="info-label">Data</span>
            <span class="info-value dim">./Data/ (portable)</span>
          </div>
        </div>

        <div class="section-hint" style="margin-top:6px">
          When single instance is on, launching a second copy focuses the existing window instead.
          When off, multiple windows can run simultaneously (useful for side-by-side comparisons — note they all share the same database).
        </div>

        {#if generalMsg}
          <div class="sync-msg" class:err={generalMsg.startsWith('Error')} style="margin-top:8px">{generalMsg}</div>
        {/if}
      {/if}

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

        <div class="info-row">
          <span class="info-label">Task text colour</span>
          <div style="display:flex;align-items:center;gap:10px">
            <ColorPicker
              value={appTaskColor}
              label="Active task font colour"
              on:change={e => { appTaskColor = e.detail; }}
            />
            <span class="info-value" style="color:{appTaskColor || 'var(--text)'}">
              Sample task text
            </span>
          </div>
        </div>

      {/if}

      <!-- SYNC TAB -->
      {#if activeTab === 'sync'}

        <!-- How it works callout -->
        <div class="sync-how">
          <div class="sync-how-icon">💡</div>
          <div>
            TaskClaw syncs by saving one file (<code>taskclaw-sync.json</code>) to a folder you choose.
            Point it at your <strong>Google Drive</strong>, <strong>OneDrive</strong>, or <strong>Dropbox</strong> folder
            and your cloud provider syncs it automatically — no accounts to connect, no passwords to enter here.
          </div>
        </div>

        <!-- Folder picker -->
        <div class="folder-row">
          <div class="folder-path" class:unset={!syncFolder}>
            {#if syncFolder}
              📁 {syncFolder}
            {:else}
              No folder chosen yet
            {/if}
          </div>
          <button class="sync-btn push" on:click={chooseFolder}>
            {syncFolder ? 'Change folder' : 'Choose folder…'}
          </button>
        </div>

        {#if syncFolder}
          {#if folderLastSync}
            <div class="info-row">
              <span class="info-label">Last synced</span>
              <span class="info-value dim">{new Date(folderLastSync).toLocaleString()}</span>
            </div>
          {/if}

          <div class="sync-actions">
            <button class="sync-btn push" on:click={push} disabled={syncing}>
              {syncing ? '…' : '↑ Save to folder'}
            </button>
            <button class="sync-btn pull" on:click={pull} disabled={syncing}>
              {syncing ? '…' : '↓ Load from folder'}
            </button>
          </div>

          <div class="section-hint" style="margin-top:4px">
            <strong>Save to folder</strong> writes your tasks to the sync file.
            <strong>Load from folder</strong> replaces your local tasks with whatever is in the file —
            use this on a second device after saving from the first.
          </div>
        {:else}
          <div class="section-hint" style="margin-top:8px">
            <strong>Where to find your cloud folder:</strong><br>
            • Google Drive for Desktop → usually <code>G:\My Drive</code> or <code>C:\Users\You\Google Drive</code><br>
            • OneDrive → usually <code>C:\Users\You\OneDrive</code><br>
            • Dropbox → usually <code>C:\Users\You\Dropbox</code><br><br>
            If you don't use cloud storage, you can pick any folder and copy the file manually.
          </div>
        {/if}

        {#if syncStatus === 'ok'}
          <div class="sync-msg">Done.</div>
        {:else if syncStatus.startsWith('Error')}
          <div class="sync-msg err">{syncStatus}</div>
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

      <!-- FORMATTING TAB -->
      {#if activeTab === 'formatting'}
        <div class="section-hint">
          Control font, color, and row styling for each task type. Changes apply immediately.
        </div>

        {#each formatSections as section}
          <details class="fmt-section">
            <summary class="fmt-summary">{section.label}</summary>
            <div class="fmt-body">

              <!-- Font group -->
              <div class="fmt-group-label">Font</div>

              <div class="info-row">
                <span class="info-label">Font family</span>
                <select
                  class="info-select"
                  value={$themeFormatting[section.key].fontFamily}
                  on:change={e => updateFmt(section.key, 'fontFamily', (e.target as HTMLSelectElement).value)}
                >
                  <option value="">Default</option>
                  <optgroup label="Sans-serif">
                    <option value="system">System UI</option>
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
                <span class="info-label">Font color</span>
                <div style="display:flex;align-items:center;gap:8px">
                  <ColorPicker
                    value={$themeFormatting[section.key].fontColor}
                    label="Font color"
                    on:change={e => updateFmt(section.key, 'fontColor', e.detail)}
                  />
                  <input
                    type="text"
                    class="hex-input"
                    placeholder="#rrggbb"
                    value={$themeFormatting[section.key].fontColor}
                    on:change={e => updateFmt(section.key, 'fontColor', (e.target as HTMLInputElement).value)}
                  />
                </div>
              </div>

              <div class="info-row">
                <span class="info-label">Style</span>
                <div style="display:flex;gap:12px;align-items:center;flex-wrap:wrap">
                  <label class="fmt-check">
                    <input type="checkbox"
                      checked={$themeFormatting[section.key].bold}
                      on:change={e => updateFmt(section.key, 'bold', (e.target as HTMLInputElement).checked)}
                    /> Bold
                  </label>
                  <label class="fmt-check">
                    <input type="checkbox"
                      checked={$themeFormatting[section.key].italic}
                      on:change={e => updateFmt(section.key, 'italic', (e.target as HTMLInputElement).checked)}
                    /> Italic
                  </label>
                  <label class="fmt-check">
                    <input type="checkbox"
                      checked={$themeFormatting[section.key].strikethrough}
                      on:change={e => updateFmt(section.key, 'strikethrough', (e.target as HTMLInputElement).checked)}
                    /> Strikethrough
                  </label>
                </div>
              </div>

              <div class="info-row">
                <span class="info-label">Underline color</span>
                <div style="display:flex;align-items:center;gap:8px">
                  <ColorPicker
                    value={$themeFormatting[section.key].underlineColor}
                    label="Underline color"
                    on:change={e => updateFmt(section.key, 'underlineColor', e.detail)}
                  />
                  <input
                    type="text"
                    class="hex-input"
                    placeholder="#rrggbb"
                    value={$themeFormatting[section.key].underlineColor}
                    on:change={e => updateFmt(section.key, 'underlineColor', (e.target as HTMLInputElement).value)}
                  />
                </div>
              </div>

              <!-- Background group -->
              <div class="fmt-group-label" style="margin-top:10px">Background</div>

              <div class="info-row">
                <span class="info-label">Background color</span>
                <div style="display:flex;align-items:center;gap:8px">
                  <ColorPicker
                    value={$themeFormatting[section.key].bgColor}
                    label="Background color"
                    on:change={e => updateFmt(section.key, 'bgColor', e.detail)}
                  />
                  <input
                    type="text"
                    class="hex-input"
                    placeholder="#rrggbb"
                    value={$themeFormatting[section.key].bgColor}
                    on:change={e => updateFmt(section.key, 'bgColor', (e.target as HTMLInputElement).value)}
                  />
                </div>
              </div>

              <!-- Highlight group -->
              <div class="fmt-group-label" style="margin-top:10px">Highlight &amp; Row</div>

              <div class="info-row">
                <span class="info-label">Highlight color</span>
                <div style="display:flex;align-items:center;gap:8px">
                  <ColorPicker
                    value={$themeFormatting[section.key].highlightColor}
                    label="Highlight color"
                    on:change={e => updateFmt(section.key, 'highlightColor', e.detail)}
                  />
                  <input
                    type="text"
                    class="hex-input"
                    placeholder="#rrggbb"
                    value={$themeFormatting[section.key].highlightColor}
                    on:change={e => updateFmt(section.key, 'highlightColor', (e.target as HTMLInputElement).value)}
                  />
                </div>
              </div>

              <div class="info-row">
                <span class="info-label">Sidebar color</span>
                <div style="display:flex;align-items:center;gap:8px">
                  <ColorPicker
                    value={$themeFormatting[section.key].sidebarColor}
                    label="Sidebar color"
                    on:change={e => updateFmt(section.key, 'sidebarColor', e.detail)}
                  />
                  <input
                    type="text"
                    class="hex-input"
                    placeholder="#rrggbb"
                    value={$themeFormatting[section.key].sidebarColor}
                    on:change={e => updateFmt(section.key, 'sidebarColor', (e.target as HTMLInputElement).value)}
                  />
                </div>
              </div>

              <div class="info-row">
                <span class="info-label">Row underline color</span>
                <div style="display:flex;align-items:center;gap:8px">
                  <ColorPicker
                    value={$themeFormatting[section.key].rowUnderlineColor}
                    label="Row underline color"
                    on:change={e => updateFmt(section.key, 'rowUnderlineColor', e.detail)}
                  />
                  <input
                    type="text"
                    class="hex-input"
                    placeholder="#rrggbb"
                    value={$themeFormatting[section.key].rowUnderlineColor}
                    on:change={e => updateFmt(section.key, 'rowUnderlineColor', (e.target as HTMLInputElement).value)}
                  />
                </div>
              </div>

              <div class="info-row">
                <span class="info-label">Row underline thickness</span>
                <div style="display:flex;align-items:center;gap:8px">
                  <input
                    type="number"
                    min="1" max="8"
                    class="name-input"
                    style="max-width:60px"
                    value={$themeFormatting[section.key].rowUnderlineThickness}
                    on:change={e => updateFmt(section.key, 'rowUnderlineThickness', Number((e.target as HTMLInputElement).value))}
                  />
                  <span class="info-value dim">px</span>
                </div>
              </div>

              <div class="info-row">
                <span class="info-label">Indent underline</span>
                <label class="fmt-check">
                  <input
                    type="checkbox"
                    checked={$themeFormatting[section.key].rowUnderlineIndent}
                    on:change={e => updateFmt(section.key, 'rowUnderlineIndent', (e.target as HTMLInputElement).checked)}
                  /> Underline starts after indent
                </label>
              </div>

            </div>
          </details>
        {/each}
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

  .general-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px 24px;
    margin-bottom: 8px;
  }

  .general-option {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    font-size: 12px;
    color: var(--text);
    cursor: pointer;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: 5px;
    background: var(--surface-elevated);
  }
  .general-option input[type=checkbox] {
    accent-color: var(--accent);
    margin-top: 2px;
    flex-shrink: 0;
  }
  .general-option span strong {
    display: block;
    font-weight: 600;
    margin-bottom: 2px;
    color: var(--text);
  }
  .general-option span {
    color: var(--text-dim);
    line-height: 1.4;
  }
  .general-option.info-only {
    cursor: default;
    flex-direction: column;
    gap: 2px;
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

  .sync-how {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    background: var(--hover);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 10px 12px;
    font-size: 11px;
    color: var(--text-dim);
    line-height: 1.6;
    margin-bottom: 14px;
  }
  .sync-how-icon { font-size: 16px; flex-shrink: 0; margin-top: 1px; }
  .sync-how strong { color: var(--text); }
  .sync-how code {
    background: var(--hover-btn);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 0 3px;
    font-size: 10px;
  }

  .folder-row {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 12px;
  }
  .folder-path {
    flex: 1;
    font-size: 11px;
    color: var(--text);
    background: var(--input-bg);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 5px 8px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  .folder-path.unset { color: var(--text-dim); font-style: italic; }

  /* ── Formatting tab ────────────────────────────────────────────────────── */
  .fmt-section {
    border: 1px solid var(--border);
    border-radius: 5px;
    margin-bottom: 6px;
    overflow: hidden;
  }
  .fmt-summary {
    cursor: pointer;
    padding: 7px 10px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text);
    background: var(--hover-btn);
    user-select: none;
    list-style: none;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .fmt-summary::-webkit-details-marker { display: none; }
  .fmt-summary::before {
    content: '▸';
    font-size: 10px;
    color: var(--text-dim);
    transition: transform 0.15s;
    display: inline-block;
  }
  details[open] > .fmt-summary::before { transform: rotate(90deg); }
  .fmt-summary:hover { background: var(--hover); }

  .fmt-body {
    padding: 8px 10px 10px;
    background: var(--surface);
  }

  .fmt-group-label {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-dim);
    margin-bottom: 4px;
    padding: 0 4px;
  }

  .fmt-check {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    cursor: pointer;
    color: var(--text);
    accent-color: var(--accent);
  }

  .hex-input {
    background: var(--input-bg);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 11px;
    width: 72px;
    outline: none;
    font-family: monospace;
  }
  .hex-input:focus { border-color: var(--accent); }
</style>
