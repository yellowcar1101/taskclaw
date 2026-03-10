<script lang="ts">
  import { showPrefs, flags, tags } from '../stores/tasks';
  import { api } from '../api';
  import type { Flag, Tag } from '../types';

  let activeTab: 'flags' | 'tags' | 'app' = 'flags';
  let error = '';

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
      <button class="tab" class:active={activeTab === 'app'}   on:click={() => activeTab = 'app'}>App</button>
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

      <!-- APP TAB -->
      {#if activeTab === 'app'}
        <div class="section-hint">App settings.</div>
        <div class="info-row">
          <span class="info-label">Version</span>
          <span class="info-value">TaskClaw 1.0</span>
        </div>
        <div class="info-row">
          <span class="info-label">Data location</span>
          <span class="info-value dim">./Data/tasks.db (portable)</span>
        </div>
        <div class="info-row">
          <span class="info-label">GDrive Sync</span>
          <span class="info-value dim">Coming soon</span>
        </div>
        <div class="info-row">
          <span class="info-label">Encryption</span>
          <span class="info-value dim">Coming soon</span>
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
  .info-label { flex: 1; color: var(--text-dim); }
  .info-value { color: var(--text); }
  .info-value.dim { color: var(--text-dim); font-style: italic; }
</style>
