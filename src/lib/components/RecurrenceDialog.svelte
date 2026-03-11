<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let rule: string | null = null;   // existing JSON or null

  const dispatch = createEventDispatcher();

  // ── Rule shape ──────────────────────────────────────────────────────────────
  interface RecurrenceRule {
    freq: 'daily' | 'weekly' | 'monthly' | 'yearly';
    interval: number;
    days_of_week?: number[];          // 0=Mon..6=Sun
    day_of_month?: number | null;     // monthly: specific day
    nth_weekday?: { n: number; day: number } | null;
    month?: number | null;            // yearly: which month
    regenerate: boolean;
    reset_subtasks: boolean;
    no_completed_copy: boolean;
  }

  const DAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
  const MONTHS = ['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];
  const NTH_LABELS = ['1st','2nd','3rd','4th','5th'];

  function defaultRule(): RecurrenceRule {
    return { freq: 'weekly', interval: 1, days_of_week: [], regenerate: false, reset_subtasks: false, no_completed_copy: false };
  }

  function parseRule(s: string | null): RecurrenceRule {
    if (!s) return defaultRule();
    try {
      const p = JSON.parse(s);
      return {
        freq: p.freq ?? 'weekly',
        interval: p.interval ?? 1,
        days_of_week: p.days_of_week ?? [],
        day_of_month: p.day_of_month ?? null,
        nth_weekday: p.nth_weekday ?? null,
        month: p.month ?? null,
        regenerate: p.regenerate ?? false,
        reset_subtasks: p.reset_subtasks ?? false,
        no_completed_copy: p.no_completed_copy ?? false,
      };
    } catch {
      return defaultRule();
    }
  }

  let r: RecurrenceRule = parseRule(rule);

  // monthly sub-mode: 'day' | 'nth'
  let monthlyMode: 'day' | 'nth' = (r.nth_weekday != null) ? 'nth' : 'day';
  $: if (r.freq !== 'monthly') { monthlyMode = r.day_of_month != null ? 'day' : 'nth'; }

  function toggleDow(idx: number) {
    const set = new Set(r.days_of_week ?? []);
    if (set.has(idx)) set.delete(idx); else set.add(idx);
    r.days_of_week = [...set].sort((a,b) => a - b);
  }

  function onFreqChange() {
    // Reset freq-specific fields on change
    r.days_of_week = [];
    r.day_of_month = null;
    r.nth_weekday = null;
    r.month = null;
    monthlyMode = 'day';
  }

  function buildRule(): string {
    const out: RecurrenceRule = {
      freq: r.freq,
      interval: Math.max(1, r.interval),
      regenerate: r.regenerate,
      reset_subtasks: r.reset_subtasks,
      no_completed_copy: r.no_completed_copy,
    };
    if (r.freq === 'weekly' && r.days_of_week && r.days_of_week.length > 0) {
      out.days_of_week = r.days_of_week;
    }
    if (r.freq === 'monthly') {
      if (monthlyMode === 'day' && r.day_of_month) {
        out.day_of_month = r.day_of_month;
      } else if (monthlyMode === 'nth' && r.nth_weekday) {
        out.nth_weekday = r.nth_weekday;
      }
    }
    if (r.freq === 'yearly' && r.month) {
      out.month = r.month;
    }
    return JSON.stringify(out);
  }

  function save() {
    dispatch('save', { rule: buildRule() });
  }

  function remove() {
    dispatch('save', { rule: null });
  }

  function cancel() {
    dispatch('cancel');
  }

  function freqLabel(freq: string, interval: number): string {
    if (interval === 1) {
      return { daily: 'day', weekly: 'week', monthly: 'month', yearly: 'year' }[freq] ?? freq;
    }
    return { daily: 'days', weekly: 'weeks', monthly: 'months', yearly: 'years' }[freq] ?? freq;
  }
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div class="overlay" on:click|self={cancel} role="none">
  <div class="dialog" role="dialog" aria-modal="true">
    <h3 class="dialog-title">Recurrence</h3>

    <!-- Frequency row -->
    <div class="field-row">
      <label class="field-label">Every</label>
      <input
        type="number" min="1" max="999"
        class="interval-input"
        bind:value={r.interval}
      />
      <select class="freq-select" bind:value={r.freq} on:change={onFreqChange}>
        <option value="daily">{freqLabel('daily', r.interval)}</option>
        <option value="weekly">{freqLabel('weekly', r.interval)}</option>
        <option value="monthly">{freqLabel('monthly', r.interval)}</option>
        <option value="yearly">{freqLabel('yearly', r.interval)}</option>
      </select>
    </div>

    <!-- Weekly: days of week -->
    {#if r.freq === 'weekly'}
      <div class="field-row">
        <label class="field-label">On</label>
        <div class="dow-row">
          {#each DAYS as day, i}
            <button
              class="dow-btn"
              class:active={(r.days_of_week ?? []).includes(i)}
              on:click={() => toggleDow(i)}
              type="button"
            >{day}</button>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Monthly: day-of-month or Nth weekday -->
    {#if r.freq === 'monthly'}
      <div class="field-row">
        <label class="field-label">Pattern</label>
        <div class="radio-group">
          <label class="radio-label">
            <input type="radio" bind:group={monthlyMode} value="day" />
            Day
            <input
              type="number" min="1" max="31"
              class="day-input"
              bind:value={r.day_of_month}
              disabled={monthlyMode !== 'day'}
            />
            of the month
          </label>
          <label class="radio-label" style="margin-top:6px">
            <input type="radio" bind:group={monthlyMode} value="nth" />
            The
            <select class="nth-select" disabled={monthlyMode !== 'nth'}
              value={r.nth_weekday?.n ?? 1}
              on:change={e => r.nth_weekday = { n: parseInt((e.target as HTMLSelectElement).value), day: r.nth_weekday?.day ?? 0 }}>
              {#each NTH_LABELS as lbl, i}
                <option value={i+1}>{lbl}</option>
              {/each}
            </select>
            <select class="dow-select" disabled={monthlyMode !== 'nth'}
              value={r.nth_weekday?.day ?? 0}
              on:change={e => r.nth_weekday = { n: r.nth_weekday?.n ?? 1, day: parseInt((e.target as HTMLSelectElement).value) }}>
              {#each DAYS as day, i}
                <option value={i}>{day}</option>
              {/each}
            </select>
            of the month
          </label>
        </div>
      </div>
    {/if}

    <!-- Yearly: month -->
    {#if r.freq === 'yearly'}
      <div class="field-row">
        <label class="field-label">In</label>
        <select class="freq-select"
          value={r.month ?? ''}
          on:change={e => r.month = (e.target as HTMLSelectElement).value ? parseInt((e.target as HTMLSelectElement).value) : null}>
          <option value="">Same month</option>
          {#each MONTHS as mon, i}
            <option value={i+1}>{mon}</option>
          {/each}
        </select>
      </div>
    {/if}

    <!-- Divider -->
    <div class="divider"></div>

    <!-- Advanced options -->
    <div class="advanced">
      <label class="chk-row">
        <input type="checkbox" bind:checked={r.regenerate} />
        Regenerate from completion date
      </label>
      <label class="chk-row">
        <input type="checkbox" bind:checked={r.reset_subtasks} />
        Reset subtasks on each recurrence
      </label>
      <label class="chk-row">
        <input type="checkbox" bind:checked={r.no_completed_copy} />
        Don't keep a completed copy
      </label>
    </div>

    <!-- Buttons -->
    <div class="btn-row">
      {#if rule}
        <button class="btn btn-danger" type="button" on:click={remove}>Remove</button>
      {/if}
      <div class="spacer"></div>
      <button class="btn btn-ghost" type="button" on:click={cancel}>Cancel</button>
      <button class="btn btn-primary" type="button" on:click={save}>Save</button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .dialog {
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 12px 40px rgba(0,0,0,0.6);
    padding: 20px;
    width: 380px;
    max-width: 95vw;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .dialog-title {
    margin: 0 0 4px;
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
  }

  .field-row {
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }

  .field-label {
    font-size: 12px;
    color: var(--text-dim);
    min-width: 52px;
    padding-top: 4px;
    flex-shrink: 0;
  }

  .interval-input {
    width: 52px;
    background: var(--input-bg);
    border: 1px solid var(--border);
    color: var(--text);
    border-radius: 4px;
    padding: 3px 6px;
    font-size: 12px;
    outline: none;
    text-align: center;
  }
  .interval-input:focus { border-color: var(--accent); }

  .freq-select, .nth-select, .dow-select {
    background: var(--input-bg);
    border: 1px solid var(--border);
    color: var(--text);
    border-radius: 4px;
    padding: 3px 6px;
    font-size: 12px;
    outline: none;
    cursor: pointer;
  }
  .freq-select:focus, .nth-select:focus, .dow-select:focus { border-color: var(--accent); }
  .freq-select { flex: 1; }
  .nth-select { width: 60px; }
  .dow-select { width: 70px; }

  .dow-row {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }

  .dow-btn {
    background: var(--input-bg);
    border: 1px solid var(--border);
    color: var(--text-dim);
    border-radius: 4px;
    padding: 3px 7px;
    font-size: 11px;
    cursor: pointer;
    transition: background 0.1s, color 0.1s, border-color 0.1s;
  }
  .dow-btn:hover { color: var(--text); background: var(--hover); }
  .dow-btn.active { background: var(--accent); color: #fff; border-color: var(--accent); }

  .radio-group { display: flex; flex-direction: column; gap: 4px; }
  .radio-label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text);
    cursor: pointer;
  }

  .day-input {
    width: 46px;
    background: var(--input-bg);
    border: 1px solid var(--border);
    color: var(--text);
    border-radius: 4px;
    padding: 2px 5px;
    font-size: 12px;
    outline: none;
    text-align: center;
  }
  .day-input:focus { border-color: var(--accent); }
  .day-input:disabled, .nth-select:disabled, .dow-select:disabled { opacity: 0.4; cursor: default; }

  .divider { height: 1px; background: var(--border); margin: 2px 0; }

  .advanced { display: flex; flex-direction: column; gap: 5px; }
  .chk-row {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
    color: var(--text);
    cursor: pointer;
  }
  .chk-row input[type="checkbox"] { flex-shrink: 0; }

  .btn-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 4px;
  }
  .spacer { flex: 1; }

  .btn {
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 5px 14px;
    font-size: 12px;
    cursor: pointer;
    transition: background 0.1s;
  }
  .btn-primary { background: var(--accent); border-color: var(--accent); color: #fff; }
  .btn-primary:hover { filter: brightness(1.15); }
  .btn-ghost { background: none; color: var(--text-dim); }
  .btn-ghost:hover { background: var(--hover); color: var(--text); }
  .btn-danger { background: none; color: var(--red); border-color: var(--red); }
  .btn-danger:hover { background: rgba(224,92,92,0.15); }
</style>
