<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { views } from '../stores/tasks';
  import { api } from '../api';
  import type { SavedView } from '../types';

  export let viewId: string;

  const dispatch = createEventDispatcher();

  $: view = $views.find(v => v.id === viewId);

  let name = '';
  let actionFilter = 'all';
  let groupBy = 'none';
  let sortBy = 'position';
  let sortDir = 'asc';
  let showCompleted = false;
  let saving = false;
  let error = '';

  // Sync draft from view
  $: if (view) {
    name = view.name;
    showCompleted = view.show_completed;
    groupBy = view.group_by || 'none';
    sortBy = view.sort_by || 'position';
    sortDir = view.sort_dir || 'asc';
    try {
      const f = JSON.parse(view.filter_json || '{}');
      actionFilter = f.action_filter ?? 'all';
    } catch { actionFilter = 'all'; }
  }

  function close() { dispatch('close'); }
  function onKeydown(e: KeyboardEvent) { if (e.key === 'Escape') close(); }

  async function save() {
    if (!view) return;
    saving = true;
    error = '';
    try {
      const updated = await api.updateView(view.id, {
        name: name.trim() || view.name,
        show_completed: showCompleted,
        group_by: groupBy,
        sort_by: sortBy,
        sort_dir: sortDir,
        visible_fields: view.visible_fields,
        filter_json: JSON.stringify({ action_filter: actionFilter }),
      });
      views.update(vs => vs.map(v => v.id === view!.id ? updated : v));
      close();
    } catch (e: any) {
      error = e?.message ?? String(e);
    } finally {
      saving = false;
    }
  }
</script>

<svelte:window on:keydown={onKeydown} />

{#if view}
  <div class="overlay" on:click|self={close} role="dialog" aria-modal="true">
    <div class="modal" role="document">
      <div class="modal-header">
        <span class="modal-title">View Settings</span>
        <button class="close-btn" on:click={close}>✕</button>
      </div>

      {#if error}
        <div class="err-bar">{error}</div>
      {/if}

      <div class="modal-body">

        <!-- Name -->
        <div class="field-row">
          <label class="field-label">Name</label>
          <input class="field-input" bind:value={name} />
        </div>

        <!-- Action Filter -->
        <div class="field-group">
          <div class="field-label">Action Filter</div>
          <div class="radio-group">
            {#each [
              ['all', 'All Tasks', 'Show all incomplete tasks'],
              ['active', 'Active', 'Leaf tasks: no blocked/future/hidden tasks'],
              ['available', 'Available', 'Active without order constraints'],
              ['next_actions', 'Next Actions', 'First active task per project'],
            ] as [val, label, desc]}
              <label class="radio-opt" class:checked={actionFilter === val}>
                <input type="radio" bind:group={actionFilter} value={val} />
                <span class="radio-label">{label}</span>
                <span class="radio-desc">{desc}</span>
              </label>
            {/each}
          </div>
        </div>

        <!-- Group By -->
        <div class="field-row">
          <label class="field-label">Group by</label>
          <select class="field-select" bind:value={groupBy}>
            <option value="none">None (flat list)</option>
            <option value="flag">Flag</option>
            <option value="due_date">Due Date</option>
            <option value="start_date">Start Date</option>
            <option value="tag">Tag</option>
          </select>
        </div>

        <!-- Sort By -->
        <div class="field-row">
          <label class="field-label">Sort by</label>
          <select class="field-select" bind:value={sortBy}>
            <option value="position">Manual Order</option>
            <option value="due_date">Due Date</option>
            <option value="start_date">Start Date</option>
            <option value="caption">Name</option>
            <option value="starred">Starred</option>
          </select>
          <select class="field-select sm" bind:value={sortDir}>
            <option value="asc">↑ Asc</option>
            <option value="desc">↓ Desc</option>
          </select>
        </div>

        <!-- Show completed -->
        <div class="field-row">
          <label class="field-label">Show completed</label>
          <label class="toggle">
            <input type="checkbox" bind:checked={showCompleted} />
            <span class="toggle-label">{showCompleted ? 'Yes' : 'No'}</span>
          </label>
        </div>

      </div>

      <div class="modal-footer">
        <button class="cancel-btn" on:click={close}>Cancel</button>
        <button class="save-btn" on:click={save} disabled={saving}>
          {saving ? 'Saving…' : 'Save'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.5);
    z-index: 850;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .modal {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    width: min(480px, 92vw);
    max-height: 85vh;
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
  .modal-title { font-size: 13px; font-weight: 600; flex: 1; }
  .close-btn { background: none; border: none; color: var(--text-dim); cursor: pointer; }
  .close-btn:hover { color: var(--text); }

  .err-bar {
    padding: 6px 14px;
    background: #E05C5C22;
    color: var(--red);
    font-size: 11px;
    border-bottom: 1px solid var(--red);
    flex-shrink: 0;
  }

  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .field-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .field-group { display: flex; flex-direction: column; gap: 8px; }

  .field-label {
    font-size: 12px;
    color: var(--text-dim);
    width: 100px;
    flex-shrink: 0;
  }

  .field-input {
    flex: 1;
    background: var(--input-bg);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 12px;
    outline: none;
  }
  .field-input:focus { border-color: var(--accent); }

  .field-select {
    flex: 1;
    background: var(--input-bg);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 4px 6px;
    border-radius: 4px;
    font-size: 12px;
    outline: none;
  }
  .field-select.sm { flex: 0 0 90px; }

  .radio-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .radio-opt {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 7px 10px;
    border-radius: 5px;
    border: 1px solid var(--border);
    cursor: pointer;
    transition: background 0.1s;
  }
  .radio-opt:hover { background: var(--hover); }
  .radio-opt.checked { border-color: var(--accent); background: var(--accent-dim); }
  .radio-opt input { margin-top: 2px; accent-color: var(--accent); }
  .radio-label { font-size: 12px; font-weight: 500; color: var(--text); min-width: 110px; }
  .radio-desc { font-size: 11px; color: var(--text-dim); }

  .toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    font-size: 12px;
    color: var(--text);
  }
  .toggle input { accent-color: var(--accent); }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 10px 16px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }

  .cancel-btn {
    background: var(--hover-btn);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 5px 14px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
  }
  .save-btn {
    background: var(--accent);
    border: none;
    color: #fff;
    padding: 5px 16px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
  }
  .save-btn:disabled { opacity: 0.5; cursor: default; }
</style>
