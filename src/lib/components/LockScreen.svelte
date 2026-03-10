<script lang="ts">
  import { api } from '../api';
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher<{ unlocked: void }>();

  let password = '';
  let error = '';
  let busy = false;

  async function unlock() {
    if (!password) return;
    busy = true;
    error = '';
    try {
      await api.unlockDb(password);
      dispatch('unlocked');
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') unlock();
  }
</script>

<div class="lock-screen">
  <div class="lock-box">
    <div class="lock-icon">🔒</div>
    <div class="lock-title">TaskClaw</div>
    <div class="lock-sub">Enter your database password to continue</div>

    <div class="field">
      <input
        class="pw-input"
        type="password"
        placeholder="Password"
        bind:value={password}
        on:keydown={onKeydown}
        autofocus
        disabled={busy}
      />
    </div>

    {#if error}
      <div class="error">{error}</div>
    {/if}

    <button class="unlock-btn" on:click={unlock} disabled={busy || !password}>
      {busy ? 'Unlocking…' : 'Unlock'}
    </button>
  </div>
</div>

<style>
  .lock-screen {
    position: fixed;
    inset: 0;
    background: var(--bg);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9999;
  }

  .lock-box {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 36px 32px 28px;
    width: 300px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    box-shadow: 0 12px 40px rgba(0,0,0,0.5);
  }

  .lock-icon {
    font-size: 32px;
    margin-bottom: 4px;
  }

  .lock-title {
    font-size: 16px;
    font-weight: 700;
    color: var(--accent);
    font-family: 'Cascadia Code', 'Fira Code', monospace;
    letter-spacing: 0.05em;
  }

  .lock-sub {
    font-size: 12px;
    color: var(--text-dim);
    font-family: sans-serif;
    text-align: center;
    margin-bottom: 4px;
  }

  .field {
    width: 100%;
  }

  .pw-input {
    width: 100%;
    background: var(--input-bg);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 7px 10px;
    border-radius: 5px;
    font-size: 13px;
    font-family: sans-serif;
    outline: none;
    box-sizing: border-box;
  }
  .pw-input:focus { border-color: var(--accent); }

  .error {
    font-size: 12px;
    color: var(--red);
    font-family: sans-serif;
    text-align: center;
  }

  .unlock-btn {
    width: 100%;
    background: var(--accent);
    color: #fff;
    border: none;
    padding: 8px;
    border-radius: 5px;
    font-size: 13px;
    font-family: sans-serif;
    cursor: pointer;
    margin-top: 4px;
    transition: filter 0.1s;
  }
  .unlock-btn:hover:not(:disabled) { filter: brightness(1.15); }
  .unlock-btn:disabled { opacity: 0.5; cursor: default; }
</style>
