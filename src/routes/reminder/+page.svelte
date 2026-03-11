<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen, emit } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import '../../app.css';

  interface ReminderItem {
    id: string;
    caption: string;
    dueLabel: string;
  }

  let reminders: ReminderItem[] = [];
  let unlistenUpdate: (() => void) | undefined;
  let unlistenClose: (() => void) | undefined;

  onMount(async () => {
    unlistenUpdate = await listen<ReminderItem[]>('reminders:update', (e) => {
      reminders = e.payload;
    });

    // Intercept native window X button — hide instead of destroy
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      const win = getCurrentWindow();
      unlistenClose = await win.onCloseRequested((event) => {
        event.preventDefault();
        invoke('hide_reminder_window');
      });
    } catch {}

    // Tell the main window we're ready to receive data
    await emit('reminder:ready', null);
  });

  onDestroy(() => {
    unlistenUpdate?.();
    unlistenClose?.();
  });

  async function dismiss(id: string) {
    await emit('reminder:dismiss', id);
    reminders = reminders.filter(r => r.id !== id);
    if (reminders.length === 0) await invoke('hide_reminder_window');
  }

  async function snooze(id: string, minutes: number) {
    await emit('reminder:snooze', { id, minutes });
    reminders = reminders.filter(r => r.id !== id);
    if (reminders.length === 0) await invoke('hide_reminder_window');
  }
</script>

<div class="reminder-page">
  {#if reminders.length === 0}
    <div class="empty">No pending reminders.</div>
  {:else}
    {#each reminders as r (r.id)}
      <div class="reminder-card">
        <div class="r-header">
          <span class="r-icon">🔔</span>
          <span class="r-title">Reminder</span>
          <button class="r-close" on:click={() => dismiss(r.id)}>✕</button>
        </div>
        <div class="r-caption">{r.caption}</div>
        {#if r.dueLabel}
          <div class="r-due">Due: {r.dueLabel}</div>
        {/if}
        <div class="r-actions">
          <button class="r-btn" on:click={() => snooze(r.id, 5)}>5 min</button>
          <button class="r-btn" on:click={() => snooze(r.id, 15)}>15 min</button>
          <button class="r-btn" on:click={() => snooze(r.id, 60)}>1 hr</button>
          <button class="r-btn dismiss" on:click={() => dismiss(r.id)}>Dismiss</button>
        </div>
      </div>
    {/each}
  {/if}
</div>

<style>
  :global(html), :global(body) {
    margin: 0;
    background: #222222;
    background: var(--surface, #222222);
    color: #e0e0e0;
    color: var(--text, #e0e0e0);
    overflow: hidden;
    font-family: system-ui, sans-serif;
  }

  .reminder-page {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    min-height: 100vh;
    box-sizing: border-box;
  }

  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100px;
    color: var(--text-dim);
    font-size: 13px;
    font-family: var(--app-font, system-ui, sans-serif);
  }

  .reminder-card {
    background: var(--surface-elevated);
    border: 1px solid var(--accent);
    border-radius: 8px;
    padding: 14px 16px;
    box-shadow: 0 4px 16px rgba(0,0,0,0.3);
    animation: slide-in 0.15s ease;
  }

  @keyframes slide-in {
    from { transform: translateY(-6px); opacity: 0; }
    to   { transform: translateY(0);    opacity: 1; }
  }

  .r-header {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 8px;
  }
  .r-icon { font-size: 15px; }
  .r-title {
    font-size: 11px;
    font-weight: 700;
    color: var(--accent);
    flex: 1;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    font-family: system-ui, sans-serif;
  }
  .r-close {
    background: none;
    border: none;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 13px;
    padding: 0 2px;
    line-height: 1;
  }
  .r-close:hover { color: var(--text); }

  .r-caption {
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
    margin-bottom: 4px;
    line-height: 1.4;
    font-family: var(--app-font, system-ui, sans-serif);
  }
  .r-due {
    font-size: 11px;
    color: var(--text-dim);
    margin-bottom: 12px;
    font-family: system-ui, sans-serif;
  }

  .r-actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .r-btn {
    background: var(--hover-btn);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 4px 10px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    font-family: system-ui, sans-serif;
  }
  .r-btn:hover { background: var(--hover); }
  .r-btn.dismiss {
    margin-left: auto;
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
  .r-btn.dismiss:hover { opacity: 0.85; }
</style>
