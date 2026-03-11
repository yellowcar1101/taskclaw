<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen, emit } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { allTasks, updateTask } from '../stores/tasks';
  import { formatDateDisplay } from '../parsing';
  import type { Task } from '../types';

  interface ReminderItem { id: string; caption: string; dueLabel: string; }

  let pendingReminders: ReminderItem[] = [];
  let interval: ReturnType<typeof setInterval>;
  let unlistenReady: (() => void) | undefined;
  let unlistenDismiss: (() => void) | undefined;
  let unlistenSnooze: (() => void) | undefined;

  // ── Snooze tracking (main window localStorage) ────────────────────────────
  function getSnoozed(): Record<string, string> {
    try { return JSON.parse(localStorage.getItem('taskclaw_snoozed') ?? '{}'); } catch { return {}; }
  }
  function setSnoozed(map: Record<string, string>) {
    localStorage.setItem('taskclaw_snoozed', JSON.stringify(map));
  }

  // ── Reminder check ────────────────────────────────────────────────────────
  async function checkReminders() {
    const now = Date.now();
    const snoozed = getSnoozed();

    // Purge expired snoozes
    let changed = false;
    for (const [id, until] of Object.entries(snoozed)) {
      if (new Date(until).getTime() <= now) { delete snoozed[id]; changed = true; }
    }
    if (changed) setSnoozed(snoozed);

    const due: ReminderItem[] = [];
    for (const task of $allTasks) {
      if (!task.reminder_at) continue;
      if (new Date(task.reminder_at).getTime() > now) continue;
      if (snoozed[task.id]) continue;
      due.push({ id: task.id, caption: task.caption, dueLabel: formatDateDisplay(task.due_date) });
    }

    if (due.length > 0) {
      pendingReminders = due;
      await invoke('show_reminder_window');
      // Give the window time to load on first open, then push data.
      // The reminder:ready event also triggers a push for fast subsequent opens.
      setTimeout(() => emit('reminders:update', pendingReminders), 700);
    }
  }

  onMount(async () => {
    checkReminders();
    interval = setInterval(checkReminders, 30_000);

    // When the reminder window finishes loading it fires reminder:ready —
    // we respond with current data so there's no race condition.
    unlistenReady = await listen('reminder:ready', () => {
      if (pendingReminders.length > 0) emit('reminders:update', pendingReminders);
    });

    // Dismiss: clear reminder_at in DB
    unlistenDismiss = await listen<string>('reminder:dismiss', async (e) => {
      const id = e.payload;
      await updateTask(id, { reminder_at: '' } as any);
      pendingReminders = pendingReminders.filter(r => r.id !== id);
    });

    // Snooze: record in localStorage
    unlistenSnooze = await listen<{ id: string; minutes: number }>('reminder:snooze', (e) => {
      const { id, minutes } = e.payload;
      const map = getSnoozed();
      map[id] = new Date(Date.now() + minutes * 60_000).toISOString();
      setSnoozed(map);
      pendingReminders = pendingReminders.filter(r => r.id !== id);
    });
  });

  onDestroy(() => {
    clearInterval(interval);
    unlistenReady?.();
    unlistenDismiss?.();
    unlistenSnooze?.();
  });

  // Re-check whenever task list changes
  $: { $allTasks; if (interval) checkReminders(); }
</script>

<!-- This component renders nothing in the main window -->
