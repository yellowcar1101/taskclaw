<script lang="ts">
  import { contexts, filterContextId, createTask, editingId } from '../stores/tasks';
  import { api } from '../api';

  let newCtxName = '';
  let newCtxColor = '#4A9EFF';
  let addingCtx = false;

  async function submitContext() {
    if (!newCtxName.trim()) return;
    await api.createContext(newCtxName.trim(), newCtxColor);
    const ctxs = await api.getContexts();
    contexts.set(ctxs);
    newCtxName = '';
    addingCtx = false;
  }
</script>

<aside class="sidebar">
  <div class="section-label">Views</div>
  <button class="nav-item" class:active={$filterContextId === null}
    on:click={() => filterContextId.set(null)}>
    ☰ All Tasks
  </button>
  <button class="nav-item starred"
    on:click={() => filterContextId.set('__starred__')}>
    ★ Starred
  </button>
  <button class="nav-item"
    on:click={() => filterContextId.set('__today__')}>
    ◷ Due Today
  </button>

  <div class="section-label" style="margin-top:12px">Contexts</div>
  {#each $contexts as ctx}
    <button
      class="nav-item ctx"
      class:active={$filterContextId === ctx.id}
      on:click={() => filterContextId.set($filterContextId === ctx.id ? null : ctx.id)}
    >
      <span class="ctx-dot" style="background:{ctx.color}"></span>
      {ctx.name}
    </button>
  {/each}

  {#if addingCtx}
    <form class="add-ctx-form" on:submit|preventDefault={submitContext}>
      <input type="color" bind:value={newCtxColor} class="color-pick" />
      <input
        class="ctx-name-input"
        bind:value={newCtxName}
        placeholder="@context"
        autofocus
        on:keydown={e => { if (e.key === 'Escape') addingCtx = false; }}
      />
      <button type="submit" class="mini-btn">✓</button>
    </form>
  {:else}
    <button class="add-ctx-btn" on:click={() => addingCtx = true}>+ context</button>
  {/if}
</aside>

<style>
  .sidebar {
    width: 160px;
    flex-shrink: 0;
    background: var(--surface);
    border-right: 1px solid var(--border);
    padding: 8px 4px;
    display: flex;
    flex-direction: column;
    gap: 1px;
    overflow-y: auto;
  }

  .section-label {
    font-size: 10px;
    font-family: sans-serif;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-dim);
    padding: 4px 8px 2px;
  }

  .nav-item {
    background: none;
    border: none;
    color: var(--text-dim);
    text-align: left;
    padding: 5px 8px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    font-family: sans-serif;
    display: flex;
    align-items: center;
    gap: 6px;
    transition: background 0.1s, color 0.1s;
    width: 100%;
  }
  .nav-item:hover { background: var(--hover); color: var(--text); }
  .nav-item.active { background: var(--selected); color: var(--accent); }
  .nav-item.starred { color: var(--gold); }

  .ctx-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .add-ctx-form {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px 4px;
  }
  .color-pick {
    width: 22px;
    height: 22px;
    border: none;
    padding: 0;
    border-radius: 3px;
    cursor: pointer;
    background: none;
  }
  .ctx-name-input {
    flex: 1;
    background: var(--input-bg);
    border: 1px solid var(--accent);
    color: var(--text);
    padding: 2px 4px;
    border-radius: 3px;
    font-size: 11px;
    outline: none;
    width: 0;
  }
  .mini-btn {
    background: var(--accent);
    border: none;
    color: #fff;
    border-radius: 3px;
    cursor: pointer;
    padding: 2px 5px;
    font-size: 11px;
  }

  .add-ctx-btn {
    background: none;
    border: none;
    color: var(--text-dim);
    font-size: 11px;
    cursor: pointer;
    padding: 4px 8px;
    text-align: left;
    font-family: sans-serif;
    transition: color 0.1s;
  }
  .add-ctx-btn:hover { color: var(--accent); }
</style>
