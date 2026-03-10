<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { allTasks, updateTask } from '../stores/tasks';
  import { formatDateDisplay } from '../parsing';
  import type { Task } from '../types';

  interface Reminder {
    task: Task;
    dueLabel: string;
  }

  let reminders: Reminder[] = [];
  let interval: ReturnType<typeof setInterval>;

  // localStorage: taskId → snoozed_until ISO string
  function getSnoozed(): Record<string, string> {
    try { return JSON.parse(localStorage.getItem('taskclaw_snoozed') ?? '{}'); } catch { return {}; }
  }
  function setSnoozed(map: Record<string, string>) {
    localStorage.setItem('taskclaw_snoozed', JSON.stringify(map));
  }

  function checkReminders() {
    const now = Date.now();
    const snoozed = getSnoozed();
    // Purge expired snoozes
    let changed = false;
    for (const [id, until] of Object.entries(snoozed)) {
      if (new Date(until).getTime() <= now) { delete snoozed[id]; changed = true; }
    }
    if (changed) setSnoozed(snoozed);

    const due: Reminder[] = [];
    for (const task of $allTasks) {
      if (!task.reminder_at) continue;
      const rms = new Date(task.reminder_at).getTime();
      if (rms > now) continue; // not yet
      if (snoozed[task.id]) continue; // snoozed
      due.push({ task, dueLabel: formatDateDisplay(task.due_date) });
    }
    reminders = due;
  }

  onMount(() => {
    checkReminders();
    interval = setInterval(checkReminders, 30_000);
  });
  onDestroy(() => clearInterval(interval));

  // Re-check whenever allTasks changes
  $: { $allTasks; checkReminders(); }

  async function dismiss(taskId: string) {
    await updateTask(taskId, { reminder_at: '' });
    reminders = reminders.filter(r => r.task.id !== taskId);
  }

  function snooze(taskId: string, minutes: number) {
    const until = new Date(Date.now() + minutes * 60_000).toISOString();
    const map = getSnoozed();
    map[taskId] = until;
    setSnoozed(map);
    reminders = reminders.filter(r => r.task.id !== taskId);
  }
</script>

{#if reminders.length > 0}
  <div class="reminder-container" role="region" aria-label="Reminders">
    {#each reminders as r (r.task.id)}
      <div class="reminder-card">
        <div class="r-header">
          <span class="r-icon">🔔</span>
          <span class="r-title">Reminder</span>
          <button class="r-close" on:click={() => dismiss(r.task.id)}>✕</button>
        </div>
        <div class="r-caption">{r.task.caption}</div>
        {#if r.dueLabel}
          <div class="r-due">Due: {r.dueLabel}</div>
        {/if}
        <div class="r-actions">
          <button class="r-btn" on:click={() => snooze(r.task.id, 5)}>5 min</button>
          <button class="r-btn" on:click={() => snooze(r.task.id, 15)}>15 min</button>
          <button class="r-btn" on:click={() => snooze(r.task.id, 60)}>1 hr</button>
          <button class="r-btn dismiss" on:click={() => dismiss(r.task.id)}>Dismiss</button>
        </div>
      </div>
    {/each}
  </div>
{/if}

<style>
  .reminder-container {
    position: fixed;
    bottom: 16px;
    right: 16px;
    z-index: 9999;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 320px;
  }

  .reminder-card {
    background: var(--surface-elevated);
    border: 1px solid var(--accent);
    border-radius: 8px;
    padding: 12px 14px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.5);
    animation: slide-in 0.2s ease;
  }

  @keyframes slide-in {
    from { transform: translateX(120%); opacity: 0; }
    to   { transform: translateX(0);   opacity: 1; }
  }

  .r-header {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 6px;
  }
  .r-icon { font-size: 14px; }
  .r-title { font-size: 11px; font-weight: 600; color: var(--accent); flex: 1; text-transform: uppercase; letter-spacing: 0.05em; }
  .r-close { background: none; border: none; color: var(--text-dim); cursor: pointer; font-size: 12px; padding: 0; }
  .r-close:hover { color: var(--text); }

  .r-caption {
    font-size: 13px;
    color: var(--text);
    font-weight: 500;
    margin-bottom: 4px;
    line-height: 1.4;
  }
  .r-due { font-size: 11px; color: var(--text-dim); margin-bottom: 10px; }

  .r-actions {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }
  .r-btn {
    background: var(--hover-btn);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 3px 8px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 11px;
  }
  .r-btn:hover { background: var(--hover); }
  .r-btn.dismiss {
    margin-left: auto;
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
</style>
