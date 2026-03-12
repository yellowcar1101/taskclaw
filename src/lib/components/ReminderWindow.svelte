<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen, emit } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { allTasks, updateTask } from '../stores/tasks';
  import type { Task } from '../types';

  interface ReminderItem {
    id: string;
    caption: string;
    flagColor: string | null;
    flagName: string | null;
    startDate: string | null;
    dueDate: string | null;
    reminderAt: string | null;
    starred: boolean;
  }

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

  // ── Reminder check — only called by the interval timer ───────────────────
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
      if (!task.reminder_at.includes('T')) continue; // require explicit time component
      if (new Date(task.reminder_at).getTime() > now) continue;
      if (snoozed[task.id]) continue;
      due.push({
        id: task.id,
        caption: task.caption,
        flagColor: task.flag?.color ?? null,
        flagName: task.flag?.name ?? null,
        startDate: task.start_date,
        dueDate: task.due_date,
        reminderAt: task.reminder_at,
        starred: task.starred,
      });
    }

    if (due.length > 0) {
      pendingReminders = due;
      await invoke('show_reminder_window');
      // Give the window time to load on first open, then push data.
      // The reminder:ready event also triggers a push for fast subsequent opens.
      setTimeout(() => emit('reminders:update', pendingReminders), 700);
    }
  }

  // ── Remove cleared reminders reactively (no show_reminder_window call) ───
  $: if (pendingReminders.length > 0) {
    const ids = new Set($allTasks.filter(t => !!t.reminder_at).map(t => t.id));
    const next = pendingReminders.filter(r => ids.has(r.id));
    if (next.length !== pendingReminders.length) {
      pendingReminders = next;
      if (next.length === 0) {
        invoke('hide_reminder_window').catch(() => {});
      } else {
        emit('reminders:update', next).catch(() => {});
      }
    }
  }

  onMount(async () => {
    // Run once on mount (not on every task change)
    checkReminders();
    interval = setInterval(checkReminders, 30_000);

    // When the reminder window finishes loading it fires reminder:ready —
    // we respond with current data so there's no race condition.
    unlistenReady = await listen('reminder:ready', () => {
      if (pendingReminders.length > 0) emit('reminders:update', pendingReminders);
    });

    // Dismiss: clear reminder_at in DB and remove from list
    unlistenDismiss = await listen<string>('reminder:dismiss', async (e) => {
      const id = e.payload;
      await updateTask(id, { reminder_at: '' } as any);
      pendingReminders = pendingReminders.filter(r => r.id !== id);
      if (pendingReminders.length === 0) {
        invoke('hide_reminder_window').catch(() => {});
      } else {
        emit('reminders:update', pendingReminders).catch(() => {});
      }
    });

    // Snooze: record in localStorage and remove from list
    unlistenSnooze = await listen<{ id: string; minutes: number }>('reminder:snooze', (e) => {
      const { id, minutes } = e.payload;
      const map = getSnoozed();
      map[id] = new Date(Date.now() + minutes * 60_000).toISOString();
      setSnoozed(map);
      pendingReminders = pendingReminders.filter(r => r.id !== id);
      if (pendingReminders.length === 0) {
        invoke('hide_reminder_window').catch(() => {});
      } else {
        emit('reminders:update', pendingReminders).catch(() => {});
      }
    });
  });

  onDestroy(() => {
    clearInterval(interval);
    unlistenReady?.();
    unlistenDismiss?.();
    unlistenSnooze?.();
  });
</script>

<!-- This component renders nothing in the main window -->
