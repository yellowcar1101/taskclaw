<script lang="ts">
  import { views, activeTabId, rightPanelOpen } from '../stores/tasks';
  import { api } from '../api';
  import type { SavedView } from '../types';

  let creating = false;
  let newName = '';
  let editingView: SavedView | null = null;

  async function createView() {
    if (!newName.trim()) return;
    const v = await api.createView({
      name: newName.trim(),
      show_completed: false,
      group_by: 'none',
      sort_by: 'start_date',
      sort_dir: 'asc',
      visible_fields: [],
    });
    views.update(vs => [...vs, v]);
    newName = '';
    creating = false;
    activeTabId.set(v.id);
  }

  async function saveViewEdit(v: SavedView) {
    if (!editingView) return;
    const updated = await api.updateView(v.id, {
      name: editingView.name,
      show_completed: editingView.show_completed,
      group_by: editingView.group_by,
      sort_by: editingView.sort_by,
      sort_dir: editingView.sort_dir,
      visible_fields: editingView.visible_fields,
    });
    views.update(vs => vs.map(x => x.id === updated.id ? updated : x));
    editingView = null;
  }

  async function removeView(id: string) {
    await api.deleteView(id);
    views.update(vs => vs.filter(v => v.id !== id));
    if ($activeTabId === id) activeTabId.set('outline');
  }

  const groupByOptions = [
    { value: 'none',       label: 'None' },
    { value: 'flag',       label: 'Flag' },
    { value: 'tag',        label: 'Tag' },
    { value: 'due_date',   label: 'Due Date' },
    { value: 'start_date', label: 'Start Date' },
    { value: 'created_at', label: 'Created Date' },
    { value: 'updated_at', label: 'Modified Date' },
  ];

  const sortByOptions = [
    { value: 'position',   label: 'Manual order' },
    { value: 'start_date', label: 'Start date' },
    { value: 'due_date',   label: 'Due date' },
    { value: 'flag',       label: 'Flag' },
    { value: 'caption',    label: 'Name' },
    { value: 'created_at', label: 'Created' },
    { value: 'updated_at', label: 'Modified' },
  ];
</script>

{#if $rightPanelOpen}
  <aside class="views-panel">
    <div class="panel-header">
      <span class="panel-title">Views</span>
      <button class="icon-btn" on:click={() => rightPanelOpen.set(false)} title="Collapse">›</button>
    </div>

    <!-- View list -->
    <div class="view-list">
      <button class="view-item" class:active={$activeTabId === 'outline'} on:click={() => activeTabId.set('outline')}>
        ☰ Outline
      </button>

      {#each $views as view (view.id)}
        <div class="view-item-wrap">
          <button
            class="view-item"
            class:active={$activeTabId === view.id}
            on:click={() => activeTabId.set(view.id)}
          >
            <span class="view-name">{view.name}</span>
          </button>
          <button class="view-edit-btn" on:click={() => { editingView = { ...view }; }} title="Edit">⚙</button>
        </div>
      {/each}
    </div>

    <!-- Add view -->
    {#if creating}
      <form class="add-form" on:submit|preventDefault={createView}>
        <input bind:value={newName} placeholder="View name…" class="add-input" autofocus />
        <button type="submit" class="mini-btn">✓</button>
        <button type="button" class="mini-btn cancel" on:click={() => creating = false}>✕</button>
      </form>
    {:else}
      <button class="add-view-btn" on:click={() => creating = true}>+ New View</button>
    {/if}

    <!-- Edit panel for a view -->
    {#if editingView}
      <div class="edit-panel">
        <div class="edit-row">
          <label>Name</label>
          <input bind:value={editingView.name} class="edit-input" />
        </div>
        <div class="edit-row">
          <label>Group by</label>
          <select bind:value={editingView.group_by}>
            {#each groupByOptions as o}<option value={o.value}>{o.label}</option>{/each}
          </select>
        </div>
        <div class="edit-row">
          <label>Sort by</label>
          <select bind:value={editingView.sort_by}>
            {#each sortByOptions as o}<option value={o.value}>{o.label}</option>{/each}
          </select>
        </div>
        <div class="edit-row">
          <label>Sort dir</label>
          <select bind:value={editingView.sort_dir}>
            <option value="asc">Ascending</option>
            <option value="desc">Descending</option>
          </select>
        </div>
        <div class="edit-row checkbox-row">
          <label>Show completed</label>
          <input type="checkbox" bind:checked={editingView.show_completed} />
        </div>
        <div class="edit-buttons">
          <button class="mini-btn" on:click={() => saveViewEdit(editingView!)}>Save</button>
          <button class="mini-btn cancel" on:click={() => editingView = null}>Cancel</button>
          <button class="mini-btn danger" on:click={() => { removeView(editingView!.id); editingView = null; }}>Delete</button>
        </div>
      </div>
    {/if}
  </aside>
{:else}
  <button class="collapsed-btn" on:click={() => rightPanelOpen.set(true)} title="Views">‹ Views</button>
{/if}

<style>
  .views-panel {
    width: 180px; flex-shrink: 0;
    background: var(--surface);
    border-left: 1px solid var(--border);
    display: flex; flex-direction: column;
    overflow: hidden;
  }

  .panel-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 6px 8px; border-bottom: 1px solid var(--border); flex-shrink: 0;
  }
  .panel-title { font-size: 10px; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-dim); font-family: sans-serif; }

  .view-list { flex: 1; overflow-y: auto; padding: 4px; }

  .view-item-wrap { display: flex; align-items: center; }
  .view-item {
    flex: 1; background: none; border: none; color: var(--text-dim);
    text-align: left; padding: 5px 8px; border-radius: 4px;
    cursor: pointer; font-size: 12px; font-family: sans-serif;
    transition: background 0.1s, color 0.1s;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  .view-item:hover { background: var(--hover); color: var(--text); }
  .view-item.active { background: var(--selected); color: var(--accent); }
  .view-name { overflow: hidden; text-overflow: ellipsis; }

  .view-edit-btn {
    background: none; border: none; color: var(--text-dim); cursor: pointer;
    padding: 3px 5px; border-radius: 3px; font-size: 11px; flex-shrink: 0;
    opacity: 0;
    transition: opacity 0.1s;
  }
  .view-item-wrap:hover .view-edit-btn { opacity: 1; }
  .view-edit-btn:hover { color: var(--text); background: var(--hover); }

  .add-view-btn {
    background: none; border: none; color: var(--text-dim); font-size: 11px;
    cursor: pointer; padding: 6px 8px; text-align: left; font-family: sans-serif;
    transition: color 0.1s; flex-shrink: 0;
  }
  .add-view-btn:hover { color: var(--accent); }

  .add-form { display: flex; align-items: center; gap: 3px; padding: 4px 6px; flex-shrink: 0; }
  .add-input {
    flex: 1; background: var(--input-bg); border: 1px solid var(--accent);
    color: var(--text); padding: 3px 6px; border-radius: 3px; font-size: 11px; outline: none;
  }

  .mini-btn {
    background: var(--hover-btn); border: 1px solid var(--border);
    color: var(--text); padding: 2px 7px; border-radius: 3px;
    cursor: pointer; font-size: 11px;
  }
  .mini-btn.cancel { color: var(--text-dim); }
  .mini-btn.danger { color: var(--red); border-color: var(--red); }

  .edit-panel {
    border-top: 1px solid var(--border);
    padding: 8px;
    display: flex; flex-direction: column; gap: 6px;
    background: var(--surface-elevated);
    flex-shrink: 0;
  }
  .edit-row { display: flex; align-items: center; gap: 6px; }
  .edit-row label { font-size: 10px; color: var(--text-dim); width: 72px; flex-shrink: 0; font-family: sans-serif; }
  .edit-input, .edit-panel select {
    flex: 1; background: var(--input-bg); border: 1px solid var(--border);
    color: var(--text); padding: 2px 5px; border-radius: 3px; font-size: 11px; outline: none;
  }
  .edit-input:focus, .edit-panel select:focus { border-color: var(--accent); }
  .checkbox-row input { width: auto; }
  .edit-buttons { display: flex; gap: 4px; flex-wrap: wrap; }

  .icon-btn {
    background: none; border: none; cursor: pointer; padding: 2px 6px;
    color: var(--text-dim); border-radius: 3px; font-size: 13px;
    transition: color 0.1s, background 0.1s;
  }
  .icon-btn:hover { color: var(--text); background: var(--hover); }

  .collapsed-btn {
    width: 24px; background: var(--surface); border: none; border-left: 1px solid var(--border);
    color: var(--text-dim); cursor: pointer; font-size: 11px; writing-mode: vertical-lr;
    padding: 10px 4px; font-family: sans-serif; letter-spacing: 0.04em;
    transition: color 0.1s;
  }
  .collapsed-btn:hover { color: var(--accent); }
</style>
