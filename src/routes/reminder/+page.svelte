<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen, emit } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import '../../app.css';

  interface ReminderItem {
    id: string;
    caption: string;
    flagColor: string | null;
    flagName: string | null;
    startDate: string | null;
    dueDate: string | null;
    starred: boolean;
  }

  let reminders: ReminderItem[] = [];
  let unlistenUpdate: (() => void) | undefined;
  let unlistenClose: (() => void) | undefined;

  function fmtDate(s: string | null): string {
    if (!s) return '—';
    const d = new Date(s + 'T00:00');
    return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
  }

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

  async function hideWindow() {
    await invoke('hide_reminder_window');
  }

  async function dismiss(id: string) {
    await emit('reminder:dismiss', id);
    reminders = reminders.filter(r => r.id !== id);
    // Main window will auto-hide if list becomes empty
  }

  async function snooze(id: string, minutes: number) {
    await emit('reminder:snooze', { id, minutes });
    reminders = reminders.filter(r => r.id !== id);
    // Main window will auto-hide if list becomes empty
  }
</script>

<div class="reminder-page">
  <!-- Header -->
  <div class="r-header">
    <span class="r-bell">🔔</span>
    <span class="r-title">Reminders ({reminders.length})</span>
    <button class="r-close-btn" on:click={hideWindow} title="Hide (reminders stay active)">✕</button>
  </div>

  <!-- List -->
  <div class="r-list">
    {#if reminders.length === 0}
      <div class="r-empty">No pending reminders.</div>
    {:else}
      {#each reminders as r (r.id)}
        <div class="r-row">
          <!-- Checkbox: checked = active, uncheck = dismiss -->
          <input
            type="checkbox"
            class="r-checkbox"
            checked={true}
            on:change={() => dismiss(r.id)}
            title="Uncheck to dismiss"
          />

          <!-- Flag color dot -->
          <span
            class="r-flag-dot"
            style={r.flagColor ? `background:${r.flagColor}` : 'background:transparent;border:1px solid #555'}
            title={r.flagName ?? ''}
          ></span>

          <!-- Star -->
          {#if r.starred}
            <span class="r-star">★</span>
          {:else}
            <span class="r-star-placeholder"></span>
          {/if}

          <!-- Task caption -->
          <span class="r-caption">{r.caption}</span>

          <!-- Start date -->
          <span class="r-date">{fmtDate(r.startDate)}</span>

          <!-- Due date -->
          <span class="r-date">{fmtDate(r.dueDate)}</span>

          <!-- Snooze buttons -->
          <div class="r-snooze-group">
            <button class="r-snooze-btn" on:click={() => snooze(r.id, 5)}>5m</button>
            <button class="r-snooze-btn" on:click={() => snooze(r.id, 15)}>15m</button>
            <button class="r-snooze-btn" on:click={() => snooze(r.id, 60)}>1h</button>
          </div>
        </div>
      {/each}
    {/if}
  </div>
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
    min-height: 100vh;
    box-sizing: border-box;
    background: #1e1e1e;
    background: var(--surface, #1e1e1e);
  }

  /* Header bar */
  .r-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    border-bottom: 1px solid #333;
    border-bottom-color: var(--border, #333);
    background: #2a2a2a;
    background: var(--surface-elevated, #2a2a2a);
    flex-shrink: 0;
  }
  .r-bell { font-size: 14px; }
  .r-title {
    font-size: 12px;
    font-weight: 700;
    color: #b0b0b0;
    color: var(--text-dim, #b0b0b0);
    flex: 1;
    font-family: system-ui, sans-serif;
    letter-spacing: 0.03em;
  }
  .r-close-btn {
    background: none;
    border: none;
    color: #777;
    color: var(--text-dim, #777);
    cursor: pointer;
    font-size: 13px;
    padding: 2px 4px;
    line-height: 1;
    border-radius: 3px;
  }
  .r-close-btn:hover {
    color: #e0e0e0;
    color: var(--text, #e0e0e0);
    background: #3a3a3a;
    background: var(--hover-btn, #3a3a3a);
  }

  /* Reminder list */
  .r-list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }

  .r-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 80px;
    color: #777;
    color: var(--text-dim, #777);
    font-size: 12px;
    font-family: system-ui, sans-serif;
  }

  /* Each reminder row */
  .r-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-bottom: 1px solid #2c2c2c;
    border-bottom-color: var(--border, #2c2c2c);
    animation: slide-in 0.12s ease;
  }
  .r-row:hover {
    background: #2a2a2a;
    background: var(--hover, #2a2a2a);
  }

  @keyframes slide-in {
    from { transform: translateX(-4px); opacity: 0; }
    to   { transform: translateX(0);    opacity: 1; }
  }

  .r-checkbox {
    flex-shrink: 0;
    width: 14px;
    height: 14px;
    accent-color: #4a9eff;
    accent-color: var(--accent, #4a9eff);
    cursor: pointer;
  }

  .r-flag-dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    flex-shrink: 0;
    display: inline-block;
  }

  .r-star {
    color: #f5a623;
    color: var(--amber, #f5a623);
    font-size: 12px;
    flex-shrink: 0;
    line-height: 1;
  }
  .r-star-placeholder {
    width: 12px;
    flex-shrink: 0;
    display: inline-block;
  }

  .r-caption {
    flex: 1;
    font-size: 13px;
    color: #e0e0e0;
    color: var(--text, #e0e0e0);
    font-family: system-ui, sans-serif;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .r-date {
    font-size: 11px;
    color: #888;
    color: var(--text-dim, #888);
    flex-shrink: 0;
    width: 46px;
    text-align: right;
    font-family: system-ui, sans-serif;
  }

  /* Snooze buttons group */
  .r-snooze-group {
    display: flex;
    gap: 3px;
    flex-shrink: 0;
  }
  .r-snooze-btn {
    background: #2e2e2e;
    background: var(--hover-btn, #2e2e2e);
    border: 1px solid #3a3a3a;
    border-color: var(--border, #3a3a3a);
    color: #aaa;
    color: var(--text-dim, #aaa);
    padding: 2px 5px;
    border-radius: 3px;
    cursor: pointer;
    font-size: 10px;
    line-height: 1.4;
    font-family: system-ui, sans-serif;
    transition: background 0.08s, color 0.08s;
  }
  .r-snooze-btn:hover {
    background: #3a3a3a;
    background: var(--hover, #3a3a3a);
    color: #e0e0e0;
    color: var(--text, #e0e0e0);
  }
</style>
