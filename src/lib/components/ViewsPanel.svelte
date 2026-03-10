<script lang="ts">
  import { views, showViewsPanel } from '../stores/tasks';
  import { api } from '../api';
  import type { SavedView } from '../types';

  let editingId: string | null = null;
  let editName = '';
  let creating = false;
  let newName = '';
  let error = '';

  function close() { showViewsPanel.set(false); }

  function startEdit(view: SavedView) {
    editingId = view.id;
    editName = view.name;
  }

  async function commitEdit(view: SavedView) {
    if (!editName.trim() || editName === view.name) { editingId = null; return; }
    try {
      const updated = await api.updateView(view.id, {
        name: editName.trim(),
        show_completed: view.show_completed,
        group_by: view.group_by,
        sort_by: view.sort_by,
        sort_dir: view.sort_dir,
        visible_fields: view.visible_fields,
        filter_json: view.filter_json,
      });
      views.update(vs => vs.map(v => v.id === view.id ? updated : v));
    } catch (e: any) { error = e?.message ?? String(e); }
    editingId = null;
  }

  async function deleteView(id: string) {
    if (!confirm('Delete this view?')) return;
    try {
      await api.deleteView(id);
      views.update(vs => vs.filter(v => v.id !== id));
    } catch (e: any) { error = e?.message ?? String(e); }
  }

  async function createView() {
    if (!newName.trim()) return;
    try {
      const view = await api.createView({
        name: newName.trim(),
        show_completed: false,
        group_by: 'none',
        sort_by: 'position',
        sort_dir: 'asc',
        visible_fields: [],
        filter_json: '{}',
      });
      views.update(vs => [...vs, view]);
      newName = '';
      creating = false;
    } catch (e: any) { error = e?.message ?? String(e); }
  }

  function onNewKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') createView();
    if (e.key === 'Escape') { creating = false; newName = ''; }
  }

  function onEditKeydown(e: KeyboardEvent, view: SavedView) {
    if (e.key === 'Enter') commitEdit(view);
    if (e.key === 'Escape') { editingId = null; }
  }

  async function toggleShowCompleted(view: SavedView) {
    try {
      const updated = await api.updateView(view.id, {
        name: view.name,
        show_completed: !view.show_completed,
        group_by: view.group_by,
        sort_by: view.sort_by,
        sort_dir: view.sort_dir,
        visible_fields: view.visible_fields,
        filter_json: view.filter_json,
      });
      views.update(vs => vs.map(v => v.id === view.id ? updated : v));
    } catch (e: any) { error = e?.message ?? String(e); }
  }
</script>

<div class="panel-overlay" on:click|self={close} role="none">
  <div class="panel" role="complementary">
    <div class="panel-header">
      <span class="panel-title">Saved Views</span>
      <button class="close-btn" on:click={close}>✕</button>
    </div>

    {#if error}
      <div class="err-bar">{error} <button on:click={() => error = ''}>✕</button></div>
    {/if}

    <div class="panel-body">
      {#each $views as view (view.id)}
        <div class="view-item">
          {#if editingId === view.id}
            <input
              class="view-name-input"
              bind:value={editName}
              on:blur={() => commitEdit(view)}
              on:keydown={e => onEditKeydown(e, view)}
              autofocus
            />
          {:else}
            <span class="view-name" on:dblclick={() => startEdit(view)} role="none">{view.name}</span>
          {/if}

          <div class="view-actions">
            <label class="chk-label" title="Show completed tasks">
              <input type="checkbox" checked={view.show_completed} on:change={() => toggleShowCompleted(view)} />
              Done
            </label>
            <button class="icon-btn" on:click={() => startEdit(view)} title="Rename">✎</button>
            <button class="icon-btn danger" on:click={() => deleteView(view.id)} title="Delete">✕</button>
          </div>
        </div>
      {/each}

      {#if $views.length === 0 && !creating}
        <div class="empty">No saved views yet.</div>
      {/if}

      {#if creating}
        <div class="new-row">
          <input
            class="view-name-input"
            bind:value={newName}
            on:keydown={onNewKeydown}
            placeholder="View name…"
            autofocus
          />
          <button class="icon-btn" on:click={createView}>✓</button>
          <button class="icon-btn" on:click={() => { creating = false; newName = ''; }}>✕</button>
        </div>
      {/if}
    </div>

    <div class="panel-footer">
      <button class="add-btn" on:click={() => creating = true} disabled={creating}>+ New View</button>
    </div>
  </div>
</div>

<style>
  .panel-overlay {
    position: fixed;
    inset: 0;
    z-index: 700;
    background: rgba(0,0,0,0.2);
    display: flex;
    justify-content: flex-end;
  }

  .panel {
    width: 260px;
    height: 100%;
    background: var(--surface);
    border-left: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    box-shadow: -8px 0 24px rgba(0,0,0,0.4);
    animation: slide-in 0.18s ease;
  }

  @keyframes slide-in {
    from { transform: translateX(100%); }
    to   { transform: translateX(0); }
  }

  .panel-header {
    display: flex;
    align-items: center;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .panel-title { font-size: 13px; font-weight: 600; flex: 1; }
  .close-btn { background: none; border: none; color: var(--text-dim); cursor: pointer; font-size: 13px; }
  .close-btn:hover { color: var(--text); }

  .err-bar {
    background: #E05C5C22;
    border-bottom: 1px solid var(--red);
    color: var(--red);
    font-size: 11px;
    padding: 6px 12px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-shrink: 0;
  }
  .err-bar button { background: none; border: none; color: var(--red); cursor: pointer; }

  .panel-body {
    flex: 1;
    overflow-y: auto;
    padding: 8px 0;
  }

  .view-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 12px;
    border-radius: 4px;
    margin: 1px 4px;
  }
  .view-item:hover { background: var(--hover); }

  .view-name {
    flex: 1;
    font-size: 12px;
    color: var(--text);
    cursor: default;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .view-name-input {
    flex: 1;
    background: var(--input-bg);
    border: 1px solid var(--accent);
    color: var(--text);
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 12px;
    outline: none;
  }

  .view-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .chk-label {
    display: flex;
    align-items: center;
    gap: 3px;
    font-size: 10px;
    color: var(--text-dim);
    cursor: pointer;
  }
  .chk-label input { accent-color: var(--accent); }

  .icon-btn {
    background: none;
    border: none;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 11px;
    padding: 2px 4px;
    border-radius: 3px;
  }
  .icon-btn:hover { color: var(--text); background: var(--hover-btn); }
  .icon-btn.danger:hover { color: var(--red); }

  .new-row {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 12px;
  }

  .empty {
    padding: 24px 16px;
    color: var(--text-dim);
    font-size: 12px;
    text-align: center;
  }

  .panel-footer {
    padding: 10px 12px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }
  .add-btn {
    width: 100%;
    background: var(--hover-btn);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 5px 10px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
  }
  .add-btn:hover:not(:disabled) { background: var(--hover); }
  .add-btn:disabled { opacity: 0.4; cursor: default; }
</style>
