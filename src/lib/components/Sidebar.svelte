<script lang="ts">
  import { flags, filterFlagId } from '../stores/tasks';
  import { api } from '../api';

  function focusOnMount(node: HTMLElement) { node.focus(); }

  let newFlagName = '';
  let newFlagColor = '#4A9EFF';
  let addingFlag = false;

  async function submitFlag() {
    if (!newFlagName.trim()) return;
    await api.createFlag(newFlagName.trim(), newFlagColor);
    const fls = await api.getFlags();
    flags.set(fls);
    newFlagName = '';
    addingFlag = false;
  }
</script>

<aside class="sidebar">
  <div class="section-label">Views</div>
  <button class="nav-item" class:active={$filterFlagId === null}
    on:click={() => filterFlagId.set(null)}>
    ☰ All Tasks
  </button>
  <button class="nav-item starred"
    on:click={() => filterFlagId.set('__starred__')}>
    ⭐ Starred
  </button>
  <button class="nav-item"
    on:click={() => filterFlagId.set('__today__')}>
    ◷ Due Today
  </button>

  <div class="section-label" style="margin-top:12px">Flags</div>
  {#each $flags as flag}
    <button
      class="nav-item flag"
      class:active={$filterFlagId === flag.id}
      on:click={() => filterFlagId.set($filterFlagId === flag.id ? null : flag.id)}
    >
      <span class="flag-dot" style="background:{flag.color}"></span>
      {flag.name}
    </button>
  {/each}

  {#if addingFlag}
    <form class="add-flag-form" on:submit|preventDefault={submitFlag}>
      <input type="color" bind:value={newFlagColor} class="color-pick" />
      <input
        class="flag-name-input"
        bind:value={newFlagName}
        placeholder="Flag name"
        use:focusOnMount
        on:keydown={e => { if (e.key === 'Escape') addingFlag = false; }}
      />
      <button type="submit" class="mini-btn">✓</button>
    </form>
  {:else}
    <button class="add-flag-btn" on:click={() => addingFlag = true}>+ flag</button>
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
    display: flex;
    align-items: center;
    gap: 6px;
    transition: background 0.1s, color 0.1s;
    width: 100%;
  }
  .nav-item:hover { background: var(--hover); color: var(--text); }
  .nav-item.active { background: var(--selected); color: var(--accent); }
  .nav-item.starred { color: var(--amber); }

  .flag-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .add-flag-form {
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
  .flag-name-input {
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
  .add-flag-btn {
    background: none;
    border: none;
    color: var(--text-dim);
    font-size: 11px;
    cursor: pointer;
    padding: 4px 8px;
    text-align: left;
    transition: color 0.1s;
  }
  .add-flag-btn:hover { color: var(--accent); }
</style>
