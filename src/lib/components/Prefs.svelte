<script lang="ts">
  import { flags, tags } from '../stores/tasks';
  import { api } from '../api';
  import type { Flag, Tag } from '../types';

  export let open = false;

  let activeTab: 'flags' | 'tags' | 'security' = 'flags';

  // Security
  let isEncrypted = false;
  let secNewPw = '';
  let secConfirmPw = '';
  let secError = '';
  let secSuccess = '';
  let secBusy = false;

  async function onSecurityTabOpen() {
    isEncrypted = await api.checkDbEncrypted();
    secNewPw = '';
    secConfirmPw = '';
    secError = '';
    secSuccess = '';
  }

  $: if (activeTab === 'security') onSecurityTabOpen();

  async function setPassword() {
    secError = '';
    secSuccess = '';
    if (!secNewPw) { secError = 'Password cannot be empty.'; return; }
    if (secNewPw !== secConfirmPw) { secError = 'Passwords do not match.'; return; }
    secBusy = true;
    try {
      await api.setDbPassword(secNewPw);
      secSuccess = 'Password set. Database is now encrypted.';
      isEncrypted = true;
      secNewPw = '';
      secConfirmPw = '';
    } catch (e: unknown) {
      secError = e instanceof Error ? e.message : String(e);
    } finally {
      secBusy = false;
    }
  }

  async function removePassword() {
    if (!confirm('Remove the database password? The file will no longer be encrypted.')) return;
    secError = '';
    secSuccess = '';
    secBusy = true;
    try {
      await api.setDbPassword('');
      secSuccess = 'Password removed. Database is now unencrypted.';
      isEncrypted = false;
    } catch (e: unknown) {
      secError = e instanceof Error ? e.message : String(e);
    } finally {
      secBusy = false;
    }
  }

  // Flags
  let newFlagName = '';
  let newFlagColor = '#4A9EFF';
  let editingFlag: Flag | null = null;

  async function createFlag() {
    if (!newFlagName.trim()) return;
    const f = await api.createFlag(newFlagName.trim(), newFlagColor);
    flags.update(fs => [...fs, f]);
    newFlagName = '';
    newFlagColor = '#4A9EFF';
  }

  async function saveFlag() {
    if (!editingFlag) return;
    const updated = await api.updateFlag(editingFlag.id, editingFlag.name, editingFlag.color);
    flags.update(fs => fs.map(f => f.id === updated.id ? updated : f));
    editingFlag = null;
  }

  async function removeFlag(id: string) {
    if (!confirm('Delete this flag? Tasks using it will have no flag.')) return;
    await api.deleteFlag(id);
    flags.update(fs => fs.filter(f => f.id !== id));
  }

  // Tags
  let newTagName = '';
  let newTagColor = '#4A9EFF';

  async function createTag() {
    if (!newTagName.trim()) return;
    const t = await api.createTag(newTagName.trim(), newTagColor);
    tags.update(ts => [...ts, t]);
    newTagName = '';
    newTagColor = '#4A9EFF';
  }

  async function removeTag(id: string) {
    await api.deleteTag(id);
    tags.update(ts => ts.filter(t => t.id !== id));
  }
</script>

{#if open}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <div class="overlay" on:click|self={() => open = false} role="none">
    <div class="modal">
      <div class="modal-header">
        <span class="modal-title">Preferences</span>
        <button class="close-btn" on:click={() => open = false}>✕</button>
      </div>

      <div class="tab-bar">
        <button class="tab" class:active={activeTab === 'flags'} on:click={() => activeTab = 'flags'}>Flags</button>
        <button class="tab" class:active={activeTab === 'tags'}  on:click={() => activeTab = 'tags'}>Tags</button>
        <button class="tab" class:active={activeTab === 'security'} on:click={() => activeTab = 'security'}>Security</button>
      </div>

      {#if activeTab === 'flags'}
        <div class="tab-content">
          <div class="item-list">
            {#each $flags as flag (flag.id)}
              {#if editingFlag?.id === flag.id}
                <div class="edit-item">
                  <input class="color-swatch" type="color" bind:value={editingFlag.color} />
                  <input class="name-input" bind:value={editingFlag.name} />
                  <button class="mini-btn" on:click={saveFlag}>✓</button>
                  <button class="mini-btn cancel" on:click={() => editingFlag = null}>✕</button>
                </div>
              {:else}
                <div class="list-item">
                  <span class="flag-dot" style="background:{flag.color}"></span>
                  <span class="item-name">{flag.name}</span>
                  <button class="icon-btn" on:click={() => editingFlag = { ...flag }} title="Edit">✎</button>
                  <button class="icon-btn danger" on:click={() => removeFlag(flag.id)} title="Delete">✕</button>
                </div>
              {/if}
            {/each}
          </div>

          <div class="add-row">
            <input class="color-swatch" type="color" bind:value={newFlagColor} />
            <input class="name-input" bind:value={newFlagName} placeholder="Flag name…"
              on:keydown={e => e.key === 'Enter' && createFlag()} />
            <button class="mini-btn primary" on:click={createFlag}>+ Add</button>
          </div>
        </div>

      {:else}
        <div class="tab-content">
          <div class="item-list">
            {#each $tags as tag (tag.id)}
              <div class="list-item">
                <span class="tag-chip" style="background:{tag.color}22;color:{tag.color};border-color:{tag.color}55">{tag.name}</span>
                <button class="icon-btn danger" on:click={() => removeTag(tag.id)} title="Delete">✕</button>
              </div>
            {/each}
          </div>

          <div class="add-row">
            <input class="color-swatch" type="color" bind:value={newTagColor} />
            <input class="name-input" bind:value={newTagName} placeholder="Tag name…"
              on:keydown={e => e.key === 'Enter' && createTag()} />
            <button class="mini-btn primary" on:click={createTag}>+ Add</button>
          </div>
        </div>
      {:else}
        <div class="tab-content">
          <div class="sec-status">
            {#if isEncrypted}
              <span class="badge encrypted">🔒 Encrypted (AES-256)</span>
            {:else}
              <span class="badge plain">🔓 Not encrypted</span>
            {/if}
          </div>

          <div class="sec-desc">
            {isEncrypted
              ? 'Your database is password-protected. Change or remove the password below.'
              : 'Set a password to encrypt your database with AES-256 (SQLCipher). You will need to enter it each time you open the app.'}
          </div>

          {#if isEncrypted}
            <div class="sec-section">
              <div class="sec-label">Change Password</div>
              <input class="name-input" type="password" placeholder="New password" bind:value={secNewPw} />
              <input class="name-input" type="password" placeholder="Confirm new password" bind:value={secConfirmPw} />
              <div class="sec-actions">
                <button class="mini-btn primary" on:click={setPassword} disabled={secBusy}>
                  {secBusy ? 'Saving…' : 'Change Password'}
                </button>
                <button class="mini-btn danger" on:click={removePassword} disabled={secBusy}>
                  Remove Password
                </button>
              </div>
            </div>
          {:else}
            <div class="sec-section">
              <div class="sec-label">Set Password</div>
              <input class="name-input" type="password" placeholder="New password" bind:value={secNewPw} />
              <input class="name-input" type="password" placeholder="Confirm password" bind:value={secConfirmPw} />
              <button class="mini-btn primary" on:click={setPassword} disabled={secBusy}>
                {secBusy ? 'Encrypting…' : 'Set Password'}
              </button>
            </div>
          {/if}

          {#if secError}
            <div class="sec-msg error">{secError}</div>
          {/if}
          {#if secSuccess}
            <div class="sec-msg success">{secSuccess}</div>
          {/if}

          <div class="sec-path">
            <span class="sec-path-label">Database file:</span>
            <code>&lt;app folder&gt;\Data\tasks.db</code>
          </div>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed; inset: 0;
    background: rgba(0,0,0,0.55);
    display: flex; align-items: center; justify-content: center;
    z-index: 1000;
  }

  .modal {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    width: 380px;
    max-height: 70vh;
    display: flex; flex-direction: column;
    overflow: hidden;
    box-shadow: 0 8px 32px rgba(0,0,0,0.5);
  }

  .modal-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .modal-title { font-size: 13px; font-weight: 600; color: var(--text); font-family: sans-serif; }
  .close-btn { background: none; border: none; color: var(--text-dim); cursor: pointer; font-size: 13px; padding: 2px 5px; border-radius: 3px; }
  .close-btn:hover { color: var(--red); }

  .tab-bar {
    display: flex; gap: 2px; padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .tab {
    background: none; border: 1px solid transparent;
    color: var(--text-dim); padding: 3px 12px; border-radius: 4px;
    cursor: pointer; font-size: 12px; font-family: sans-serif;
    transition: background 0.1s, color 0.1s;
  }
  .tab:hover { background: var(--hover); color: var(--text); }
  .tab.active { background: var(--selected); color: var(--accent); border-color: var(--accent-dim); }

  .tab-content { flex: 1; overflow-y: auto; padding: 8px 10px; display: flex; flex-direction: column; gap: 6px; }

  .item-list { display: flex; flex-direction: column; gap: 4px; }

  .list-item { display: flex; align-items: center; gap: 8px; padding: 4px 4px; border-radius: 4px; }
  .list-item:hover { background: var(--hover); }
  .item-name { flex: 1; font-size: 13px; color: var(--text); font-family: sans-serif; }

  .edit-item { display: flex; align-items: center; gap: 6px; padding: 2px 4px; }

  .flag-dot { width: 10px; height: 10px; border-radius: 50%; flex-shrink: 0; }

  .color-swatch { width: 28px; height: 24px; padding: 0; border: 1px solid var(--border); border-radius: 3px; cursor: pointer; flex-shrink: 0; }

  .name-input {
    flex: 1; background: var(--input-bg); border: 1px solid var(--border);
    color: var(--text); padding: 3px 6px; border-radius: 3px; font-size: 12px; outline: none;
  }
  .name-input:focus { border-color: var(--accent); }

  .add-row { display: flex; align-items: center; gap: 6px; padding: 6px 4px; border-top: 1px solid var(--border); margin-top: 4px; }

  .mini-btn {
    background: var(--hover-btn); border: 1px solid var(--border);
    color: var(--text); padding: 3px 8px; border-radius: 3px;
    cursor: pointer; font-size: 11px; white-space: nowrap; font-family: sans-serif;
  }
  .mini-btn.cancel { color: var(--text-dim); }
  .mini-btn.primary { background: var(--accent); color: #fff; border-color: var(--accent); }
  .mini-btn.primary:hover { filter: brightness(1.1); }

  .icon-btn { background: none; border: none; color: var(--text-dim); cursor: pointer; font-size: 12px; padding: 2px 4px; border-radius: 3px; }
  .icon-btn:hover { color: var(--text); background: var(--hover-btn); }
  .icon-btn.danger:hover { color: var(--red); }

  .tag-chip { font-size: 11px; padding: 2px 8px; border-radius: 10px; border: 1px solid; white-space: nowrap; font-family: sans-serif; flex: 1; }

  .sec-status { display: flex; align-items: center; gap: 6px; }
  .badge { font-size: 12px; font-family: sans-serif; padding: 3px 10px; border-radius: 12px; }
  .badge.encrypted { background: #1a3a1a; color: #6abf69; border: 1px solid #6abf6944; }
  .badge.plain { background: #2a2a1a; color: #d4a843; border: 1px solid #d4a84344; }

  .sec-desc { font-size: 12px; color: var(--text-dim); font-family: sans-serif; line-height: 1.5; }

  .sec-section { display: flex; flex-direction: column; gap: 6px; }
  .sec-label { font-size: 11px; color: var(--text-dim); font-family: sans-serif; text-transform: uppercase; letter-spacing: 0.05em; }

  .sec-actions { display: flex; gap: 6px; }

  .mini-btn.danger { color: var(--red, #e05c5c); }
  .mini-btn.danger:hover { background: #3a1a1a; }

  .sec-msg { font-size: 12px; font-family: sans-serif; padding: 5px 8px; border-radius: 4px; }
  .sec-msg.error { background: #3a1a1a; color: var(--red, #e05c5c); }
  .sec-msg.success { background: #1a3a1a; color: #6abf69; }

  .sec-path { font-size: 11px; color: var(--text-dim); font-family: sans-serif; margin-top: 4px; display: flex; flex-direction: column; gap: 3px; }
  .sec-path-label { font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; }
  .sec-path code { font-size: 11px; background: var(--surface-elevated); padding: 3px 6px; border-radius: 3px; word-break: break-all; }
</style>
