<script lang="ts">
  import { get } from 'svelte/store';
  import { detailTaskId, taskById, tags, flags, updateTask, allTasks } from '../stores/tasks';
  import { api } from '../api';
  import type { Task } from '../types';
  import { parseCaption } from '../parsing';
  import RecurrenceDialog from './RecurrenceDialog.svelte';

  $: task = $detailTaskId ? $taskById.get($detailTaskId) ?? null : null;

  // ── Section collapse state ─────────────────────────────────────────────────
  let collapsed = { notes: false, general: false, timing: false, format: false };

  // ── Caption editing ─────────────────────────────────────────────────────────
  let captionEl: HTMLHeadingElement;
  async function saveCaption() {
    if (!task || !captionEl) return;
    const txt = captionEl.innerText.trim();
    if (txt && txt !== task.caption) await updateTask(task.id, { caption: txt });
  }
  function onCaptionKey(e: KeyboardEvent) {
    if (e.key === 'Enter') { e.preventDefault(); captionEl?.blur(); }
  }

  // ── Notes ───────────────────────────────────────────────────────────────────
  let noteEditing = false;
  let noteValue = '';
  let noteEl: HTMLTextAreaElement;

  // ── Notes resize ─────────────────────────────────────────────────────────
  let notesHeight = parseInt(localStorage.getItem('notes_height') ?? '120');
  let resizing = false;
  let resizeStartY = 0;
  let resizeStartH = 0;

  function onResizeStart(e: MouseEvent) {
    resizing = true;
    resizeStartY = e.clientY;
    resizeStartH = notesHeight;
    window.addEventListener('mousemove', onResizeMove);
    window.addEventListener('mouseup', onResizeEnd);
    e.preventDefault();
  }
  function onResizeMove(e: MouseEvent) {
    if (!resizing) return;
    notesHeight = Math.max(60, resizeStartH + (e.clientY - resizeStartY));
    localStorage.setItem('notes_height', String(notesHeight));
  }
  function onResizeEnd() {
    resizing = false;
    window.removeEventListener('mousemove', onResizeMove);
    window.removeEventListener('mouseup', onResizeEnd);
  }

  $: if (task) noteValue = task.note ?? '';

  async function saveNote() {
    if (!task) return;
    noteEditing = false;
    if (noteValue !== task.note) await updateTask(task.id, { note: noteValue });
  }

  function wrap(before: string, after: string, placeholder = '') {
    const el = noteEl;
    if (!el) return;
    const start = el.selectionStart;
    const end = el.selectionEnd;
    const selected = noteValue.slice(start, end) || placeholder;
    const replacement = before + selected + after;
    noteValue = noteValue.slice(0, start) + replacement + noteValue.slice(end);
    setTimeout(() => {
      el.focus();
      const newStart = start + before.length;
      const newEnd = newStart + selected.length;
      el.setSelectionRange(newStart, newEnd);
    }, 0);
  }

  function insertLink() {
    const el = noteEl;
    if (!el) return;
    const start = el.selectionStart;
    const end = el.selectionEnd;
    const selected = noteValue.slice(start, end);
    const url = prompt('URL:');
    if (!url) return;
    const text = selected || 'link text';
    const replacement = `[${text}](${url})`;
    noteValue = noteValue.slice(0, start) + replacement + noteValue.slice(end);
    setTimeout(() => { el.focus(); el.setSelectionRange(start + 1, start + 1 + text.length); }, 0);
  }

  function renderMarkdown(md: string): string {
    if (!md) return '<span style="color:var(--text-dim);font-style:italic">No notes</span>';
    let h = md
      .replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;')
      .replace(/^### (.+)$/gm, '<h3>$1</h3>')
      .replace(/^## (.+)$/gm, '<h2>$1</h2>')
      .replace(/^# (.+)$/gm, '<h1>$1</h1>')
      .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
      .replace(/\*(.+?)\*/g, '<em>$1</em>')
      .replace(/~~(.+?)~~/g, '<del>$1</del>')
      .replace(/`(.+?)`/g, '<code>$1</code>')
      .replace(/\[(.+?)\]\((.+?)\)/g, (_, text, url) => {
        const trimmed = url.trim();
        const safe = /^(https?:|mailto:)/i.test(trimmed) ? trimmed : '#';
        return `<a href="${safe}" target="_blank" rel="noopener noreferrer">${text}</a>`;
      })
      .replace(/^[-*] (.+)$/gm, '<li>$1</li>')
      .replace(/(<li>.*<\/li>)/gs, '<ul>$1</ul>')
      .replace(/\n\n/g, '</p><p>')
      .replace(/\n/g, '<br>');
    return `<p>${h}</p>`;
  }

  // ── General section ─────────────────────────────────────────────────────────
  async function toggleBool(field: string, val: boolean) {
    if (!task) return;
    await updateTask(task.id, { [field]: val });
  }

  let tagInput = '';
  let tagDropdown: string[] = [];

  $: if (tagInput.length > 0) {
    const lower = tagInput.toLowerCase();
    tagDropdown = $tags.filter(t => t.name.toLowerCase().includes(lower) && !task?.tags.some(tt => tt.id === t.id)).map(t => t.id);
  } else {
    tagDropdown = [];
  }

  async function addTag(tagId: string) {
    if (!task) return;
    const current = task.tags.map(t => t.id);
    if (!current.includes(tagId)) await updateTask(task.id, { tag_ids: [...current, tagId] });
    tagInput = '';
    tagDropdown = [];
  }

  async function createAndAddTag() {
    if (!task || !tagInput.trim()) return;
    const tag = await api.createTag(tagInput.trim(), '#888888');
    tags.update(ts => [...ts, tag]);
    await addTag(tag.id);
  }

  async function removeTag(tagId: string) {
    if (!task) return;
    await updateTask(task.id, { tag_ids: task.tags.filter(t => t.id !== tagId).map(t => t.id) });
  }

  async function onTagKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      if (tagDropdown.length > 0) await addTag(tagDropdown[0]);
      else await createAndAddTag();
    }
    if (e.key === 'Escape') { tagInput = ''; tagDropdown = []; }
  }

  // ── Timing section ──────────────────────────────────────────────────────────
  let useStartTime = false;
  let useDueTime = false;
  let startDateVal = '';
  let startTimeVal = '';
  let dueDateVal = '';
  let dueTimeVal = '';
  let reminderOn = false;
  let reminderDateVal = '';
  let reminderTimeVal = '';

  $: if (task) {
    const sd = task.start_date ?? '';
    const dd = task.due_date ?? '';
    const ra = task.reminder_at ?? '';
    useStartTime = sd.includes('T');
    useDueTime   = dd.includes('T');
    startDateVal  = sd.split('T')[0];
    startTimeVal  = sd.includes('T') ? sd.split('T')[1].slice(0,5) : '';
    dueDateVal    = dd.split('T')[0];
    dueTimeVal    = dd.includes('T') ? dd.split('T')[1].slice(0,5) : '';
    reminderOn    = !!ra;
    reminderDateVal = ra.split('T')[0];
    reminderTimeVal = ra.includes('T') ? ra.split('T')[1].slice(0,5) : '';
  }

  function nowTime(): string {
    const d = new Date();
    return d.toTimeString().slice(0, 5);
  }

  function buildDateStr(date: string, time: string, withTime: boolean): string {
    if (!date) return '';
    if (withTime && time) return `${date}T${time}`;
    return date;
  }

  async function saveStartDate() {
    if (!task) return;
    const val = buildDateStr(startDateVal, startTimeVal, useStartTime);
    await updateTask(task.id, { start_date: val });
  }
  async function saveDueDate() {
    if (!task) return;
    const val = buildDateStr(dueDateVal, dueTimeVal, useDueTime);
    await updateTask(task.id, { due_date: val });
  }
  async function saveReminder() {
    if (!task) return;
    if (!reminderOn) { await updateTask(task.id, { reminder_at: '' }); return; }
    // Pre-populate date from due_date or start_date when first enabling
    if (!reminderDateVal) {
      reminderDateVal = (task.due_date ?? task.start_date ?? '').split('T')[0];
    }
    if (!reminderTimeVal) {
      reminderTimeVal = '09:00';
    }
    const val = buildDateStr(reminderDateVal, reminderTimeVal, true);
    if (val) await updateTask(task.id, { reminder_at: val });
  }

  function quickStart(n: number) {
    const d = new Date();
    d.setDate(d.getDate() + n);
    startDateVal = d.toISOString().split('T')[0];
    saveStartDate();
  }

  // ── Recurrence ──────────────────────────────────────────────────────────────
  let showRecurrenceDialog = false;

  function recurrenceSummary(ruleStr: string | null): string {
    if (!ruleStr) return '';
    try {
      const r = JSON.parse(ruleStr);
      const interval = r.interval ?? 1;
      const DAYS = ['Mon','Tue','Wed','Thu','Fri','Sat','Sun'];
      const NTH = ['','1st','2nd','3rd','4th','5th'];
      const MONTHS = ['','Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];
      let s = `Every ${interval > 1 ? interval + ' ' : ''}`;
      if (r.freq === 'daily')   s += interval === 1 ? 'day' : 'days';
      if (r.freq === 'weekly') {
        s += interval === 1 ? 'week' : 'weeks';
        if (r.days_of_week?.length) s += ' on ' + r.days_of_week.map((d: number) => DAYS[d]).join(', ');
      }
      if (r.freq === 'monthly') {
        s += interval === 1 ? 'month' : 'months';
        if (r.day_of_month)  s += `, day ${r.day_of_month}`;
        if (r.nth_weekday)   s += `, ${NTH[r.nth_weekday.n]} ${DAYS[r.nth_weekday.day]}`;
      }
      if (r.freq === 'yearly') {
        s += interval === 1 ? 'year' : 'years';
        if (r.month) s += ' in ' + MONTHS[r.month];
      }
      if (r.regenerate) s += ' (regenerate)';
      return s;
    } catch { return 'Recurring'; }
  }

  async function saveRecurrence(e: CustomEvent<{ rule: string | null }>) {
    if (!task) return;
    showRecurrenceDialog = false;
    await updateTask(task.id, { recurrence_rule: e.detail.rule ?? '' });
  }

  // ── Format section ──────────────────────────────────────────────────────────
  let customFmt = { bold: false, italic: false, underline: false, strikethrough: false,
                    highlight_color: null as string|null, font_color: null as string|null,
                    sidebar_color: null as string|null, subtasks_inherit: false };
  let useCustomFmt = false;
  let prevFmtTaskId: string | null = null;

  $: if (task && task.id !== prevFmtTaskId) {
    prevFmtTaskId = task.id;
    if (task.custom_format) {
      try {
        customFmt = { bold: false, italic: false, underline: false, strikethrough: false,
                      highlight_color: null, font_color: null, sidebar_color: null, subtasks_inherit: false,
                      ...JSON.parse(task.custom_format) };
        useCustomFmt = true;
      } catch { useCustomFmt = false; }
    } else {
      customFmt = { bold: false, italic: false, underline: false, strikethrough: false,
                    highlight_color: null, font_color: null, sidebar_color: null, subtasks_inherit: false };
      useCustomFmt = false;
    }
  }

  async function saveFormat() {
    if (!task) return;
    const val = useCustomFmt ? JSON.stringify(customFmt) : '';
    await updateTask(task.id, { custom_format: val });
  }
</script>

{#if task}
<div class="detail-panel">
  <!-- Header -->
  <div class="detail-header">
    <h2
      class="detail-caption"
      contenteditable="true"
      bind:this={captionEl}
      on:blur={saveCaption}
      on:keydown={onCaptionKey}
    >{task.caption}</h2>
    <button class="close-btn" on:click={() => detailTaskId.set(null)}>✕</button>
  </div>

  <div class="detail-body">

    <!-- Notes section -->
    <div class="section">
      <button class="sec-header" on:click={() => collapsed.notes = !collapsed.notes}>
        <span class="sec-arrow">{collapsed.notes ? '▶' : '▼'}</span>
        Notes
      </button>
      {#if !collapsed.notes}
        <div class="sec-body">
          {#if noteEditing}
            <div class="note-toolbar">
              <button class="nt-btn" on:click={() => wrap('**', '**', 'bold')} title="Bold"><strong>B</strong></button>
              <button class="nt-btn" on:click={() => wrap('*', '*', 'italic')} title="Italic"><em>I</em></button>
              <button class="nt-btn" on:click={() => wrap('~~', '~~', 'text')} title="Strikethrough"><s>S</s></button>
              <button class="nt-btn" on:click={() => wrap('`', '`', 'code')} title="Inline code" style="font-family:monospace">``</button>
              <button class="nt-btn" on:click={insertLink} title="Insert link">🔗</button>
              <div class="nt-sep"></div>
              <button class="nt-btn" on:click={() => wrap('# ', '', 'Heading')} title="Heading">H</button>
              <button class="nt-btn" on:click={() => wrap('- ', '', 'item')} title="List item">•</button>
            </div>
            <textarea
              class="note-editor"
              bind:this={noteEl}
              bind:value={noteValue}
              on:blur={saveNote}
              placeholder="Add notes…"
              style="height:{notesHeight}px; resize:none;"
            ></textarea>
            <div class="notes-resize-handle" on:mousedown={onResizeStart} role="separator"></div>
          {:else}
            <div
              class="note-preview"
              on:click={() => { noteEditing = true; noteValue = task?.note ?? ''; }}
              role="button"
              tabindex="0"
              on:keydown={e => { if (e.key === 'Enter' || e.key === ' ') noteEditing = true; }}
            >
              {@html renderMarkdown(task.note)}
            </div>
          {/if}
        </div>
      {/if}
    </div>

    <!-- General section -->
    <div class="section">
      <button class="sec-header" on:click={() => collapsed.general = !collapsed.general}>
        <span class="sec-arrow">{collapsed.general ? '▶' : '▼'}</span>
        General
      </button>
      {#if !collapsed.general}
        <div class="sec-body">
          <label class="chk-row">
            <input type="checkbox" checked={task.is_folder}
              on:change={e => toggleBool('is_folder', (e.target as HTMLInputElement).checked)} />
            Folder
          </label>
          <label class="chk-row">
            <input type="checkbox" checked={task.is_project}
              on:change={e => toggleBool('is_project', (e.target as HTMLInputElement).checked)} />
            Project
          </label>
          <label class="chk-row">
            <input type="checkbox" checked={task.hide_in_views}
              on:change={e => toggleBool('hide_in_views', (e.target as HTMLInputElement).checked)} />
            Hide in Views
          </label>
          <label class="chk-row">
            <input type="checkbox" checked={task.subtasks_in_order}
              on:change={e => toggleBool('subtasks_in_order', (e.target as HTMLInputElement).checked)} />
            Complete in Order
          </label>
          <label class="chk-row">
            <input type="checkbox" checked={task.starred}
              on:change={e => toggleBool('starred', (e.target as HTMLInputElement).checked)} />
            Starred ⭐
          </label>

          <!-- Flag picker -->
          <div class="field-row" style="margin-top:8px">
            <span class="field-label">Flag</span>
            <select class="field-select flag-select"
              value={task.flag_id ?? ''}
              on:change={e => updateTask(task!.id, { flag_id: (e.target as HTMLSelectElement).value })}
            >
              <option value="">— none —</option>
              {#each $flags as flag}
                <option value={flag.id}>{flag.name}</option>
              {/each}
            </select>
          </div>

          <!-- Tags -->
          <div class="field-row" style="margin-top:8px;align-items:flex-start;flex-direction:column;gap:4px">
            <span class="field-label">Tags</span>
            <div class="tag-chips tag-chips-compact">
              {#each task.tags as tag}
                <span class="tag-chip" style="background:{tag.color}22;color:{tag.color};border-color:{tag.color}44">
                  {tag.name}
                  <button class="chip-rm" on:click={() => removeTag(tag.id)}>✕</button>
                </span>
              {/each}
            </div>
            <div style="position:relative;width:100%">
              <input
                class="tag-input"
                bind:value={tagInput}
                on:keydown={onTagKeydown}
                on:blur={() => setTimeout(() => { tagInput = ''; tagDropdown = []; }, 150)}
                placeholder="Add tag…"
              />
              {#if tagDropdown.length > 0}
                <div class="tag-dropdown">
                  {#each tagDropdown as tid}
                    {@const t = $tags.find(x => x.id === tid)}
                    {#if t}
                      <div class="tag-opt" on:mousedown|preventDefault={() => addTag(tid)} role="option" aria-selected="false" tabindex="0" on:keydown>
                        <span style="width:8px;height:8px;border-radius:50%;background:{t.color};display:inline-block;margin-right:4px"></span>
                        {t.name}
                      </div>
                    {/if}
                  {/each}
                </div>
              {/if}
            </div>
          </div>
        </div>
      {/if}
    </div>

    <!-- Timing section -->
    <div class="section">
      <button class="sec-header" on:click={() => collapsed.timing = !collapsed.timing}>
        <span class="sec-arrow">{collapsed.timing ? '▶' : '▼'}</span>
        Timing & Reminder
      </button>
      {#if !collapsed.timing}
        <div class="sec-body">
          <div class="quick-dates">
            <button class="quick-btn" on:click={() => quickStart(0)}>Today</button>
            <button class="quick-btn" on:click={() => quickStart(1)}>Tomorrow</button>
            <button class="quick-btn" on:click={() => quickStart(7)}>Next week</button>
          </div>

          <div class="date-row">
            <span class="field-label" style="width:48px">Start</span>
            <input type="date" class="date-input" bind:value={startDateVal} on:change={saveStartDate} />
            {#if useStartTime}
              <input type="time" class="time-input" bind:value={startTimeVal} on:change={saveStartDate} />
              <button class="time-clear" on:click={() => { useStartTime = false; startTimeVal = ''; saveStartDate(); }} title="Remove time">✕</button>
            {:else if startDateVal}
              <button class="time-add" on:click={() => { useStartTime = true; startTimeVal = nowTime(); saveStartDate(); }} title="Add time">⊕ time</button>
            {/if}
          </div>

          <div class="date-row">
            <span class="field-label" style="width:48px">Due</span>
            <input type="date" class="date-input" bind:value={dueDateVal} on:change={saveDueDate} />
            {#if useDueTime}
              <input type="time" class="time-input" bind:value={dueTimeVal} on:change={saveDueDate} />
              <button class="time-clear" on:click={() => { useDueTime = false; dueTimeVal = ''; saveDueDate(); }} title="Remove time">✕</button>
            {:else if dueDateVal}
              <button class="time-add" on:click={() => { useDueTime = true; dueTimeVal = nowTime(); saveDueDate(); }} title="Add time">⊕ time</button>
            {/if}
          </div>

          <div class="date-row" style="margin-top:8px">
            <label class="chk-row" style="margin:0">
              <input type="checkbox" bind:checked={reminderOn} on:change={saveReminder} />
              Reminder
            </label>
          </div>
          {#if reminderOn}
            <div class="date-row">
              <input type="date" class="date-input" bind:value={reminderDateVal} on:change={saveReminder} />
              <input type="time" class="time-input" bind:value={reminderTimeVal} on:change={saveReminder} />
            </div>
          {/if}

          <button class="recurrence-btn" on:click={() => showRecurrenceDialog = true}>
            ↺ {task?.recurrence_rule ? 'Edit recurrence' : 'Recurrence…'}
          </button>
          {#if task?.recurrence_rule}
            <div class="recurrence-summary">{recurrenceSummary(task.recurrence_rule)}</div>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Format section -->
    <div class="section">
      <button class="sec-header" on:click={() => collapsed.format = !collapsed.format}>
        <span class="sec-arrow">{collapsed.format ? '▶' : '▼'}</span>
        Format
      </button>
      {#if !collapsed.format}
        <div class="sec-body">
          <label class="chk-row" style="margin-bottom:8px">
            <input type="checkbox" bind:checked={useCustomFmt} on:change={saveFormat} />
            Use custom formatting
          </label>
          <div class="fmt-row" class:disabled={!useCustomFmt}>
            <label class="fmt-btn" class:active={customFmt.bold} title="Bold">
              <input type="checkbox" bind:checked={customFmt.bold} on:change={saveFormat} hidden />
              <strong>B</strong>
            </label>
            <label class="fmt-btn" class:active={customFmt.italic} title="Italic">
              <input type="checkbox" bind:checked={customFmt.italic} on:change={saveFormat} hidden />
              <em>I</em>
            </label>
            <label class="fmt-btn" class:active={customFmt.underline} title="Underline">
              <input type="checkbox" bind:checked={customFmt.underline} on:change={saveFormat} hidden />
              <u>U</u>
            </label>
            <label class="fmt-btn" class:active={customFmt.strikethrough} title="Strikethrough">
              <input type="checkbox" bind:checked={customFmt.strikethrough} on:change={saveFormat} hidden />
              <s>S</s>
            </label>
            <label class="fmt-color" title="Font color">
              A
              <input type="color" bind:value={customFmt.font_color} on:change={saveFormat}
                style="opacity:0;position:absolute;width:0;height:0" />
            </label>
          </div>
          <label class="chk-row" style="margin-top:6px">
            <input type="checkbox" bind:checked={customFmt.subtasks_inherit} on:change={saveFormat}
              disabled={!useCustomFmt} />
            Subtasks inherit format
          </label>
        </div>
      {/if}
    </div>

  </div>
</div>
{/if}

{#if showRecurrenceDialog && task}
  <RecurrenceDialog
    rule={task.recurrence_rule}
    on:save={saveRecurrence}
    on:cancel={() => showRecurrenceDialog = false}
  />
{/if}

<style>
  .detail-panel {
    width: 320px;
    flex-shrink: 0;
    background: var(--surface);
    border-left: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .detail-header {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 10px 12px 8px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .detail-caption {
    flex: 1;
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
    line-height: 1.4;
    min-height: 20px;
    outline: none;
    word-break: break-word;
  }
  .detail-caption:focus { color: var(--text); }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 12px;
    padding: 2px 4px;
    border-radius: 3px;
    flex-shrink: 0;
  }
  .close-btn:hover { color: var(--text); background: var(--hover); }

  .detail-body {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }

  .section { border-bottom: 1px solid var(--border); }

  .sec-header {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 6px;
    background: none;
    border: none;
    color: var(--text-dim);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    cursor: pointer;
    padding: 6px 12px;
    text-align: left;
  }
  .sec-header:hover { color: var(--text); background: var(--hover); }
  .sec-arrow { font-size: 9px; }

  .sec-body {
    padding: 6px 12px 10px;
  }

  /* Notes */
  .note-preview {
    font-size: 12px;
    color: var(--text);
    line-height: 1.5;
    cursor: text;
    min-height: 40px;
    padding: 4px;
    border-radius: 3px;
  }
  .note-preview:hover { background: var(--hover); }
  .note-preview :global(h1), .note-preview :global(h2), .note-preview :global(h3) {
    font-size: 13px; font-weight: 600; margin: 4px 0;
  }
  .note-preview :global(ul) { padding-left: 16px; margin: 4px 0; }
  .note-preview :global(a) { color: var(--accent); }
  .note-preview :global(code) { background: var(--input-bg); padding: 1px 4px; border-radius: 2px; font-size: 11px; }
  .note-preview :global(p) { margin: 4px 0; }

  .note-toolbar {
    display: flex;
    align-items: center;
    gap: 1px;
    padding: 3px 2px;
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-bottom: none;
    border-radius: 3px 3px 0 0;
  }
  .nt-btn {
    background: none;
    border: none;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 3px;
    line-height: 1.4;
  }
  .nt-btn:hover { color: var(--text); background: var(--hover); }
  .nt-sep { width: 1px; height: 14px; background: var(--border); margin: 0 3px; }

  .note-editor {
    width: 100%;
    background: var(--input-bg);
    border: 1px solid var(--accent);
    border-radius: 0 0 3px 3px !important;
    color: var(--text);
    font-family: 'Cascadia Code', monospace;
    font-size: 12px;
    padding: 6px;
    border-radius: 3px;
    resize: none;
    outline: none;
    line-height: 1.5;
    box-sizing: border-box;
  }

  .notes-resize-handle {
    height: 6px;
    background: transparent;
    cursor: ns-resize;
    display: flex;
    align-items: center;
    justify-content: center;
    margin: 0;
    border-top: 2px solid var(--border);
  }
  .notes-resize-handle::after {
    content: '';
    width: 32px;
    height: 3px;
    border-top: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
  }
  .notes-resize-handle:hover {
    border-top-color: var(--accent);
  }

  /* General */
  .chk-row {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text);
    cursor: pointer;
    padding: 2px 0;
  }
  .chk-row input[type=checkbox] { accent-color: var(--accent); width: 13px; height: 13px; }

  .field-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
  }
  .field-label { color: var(--text-dim); font-size: 11px; flex-shrink: 0; }
  .field-select {
    flex: 1;
    background: var(--input-bg);
    border: 1px solid var(--border);
    color: var(--text);
    border-radius: 3px;
    padding: 2px 4px;
    font-size: 12px;
    outline: none;
  }
  .flag-select {
    flex: 0 1 auto;
    max-width: 180px;
    width: fit-content;
  }
  .tag-chips-compact {
    max-width: 100%;
  }

  .tag-chips { display: flex; flex-wrap: wrap; gap: 4px; }
  .tag-chip {
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 10px;
    border: 1px solid;
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .chip-rm {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 9px;
    padding: 0;
    color: inherit;
    opacity: 0.7;
  }
  .chip-rm:hover { opacity: 1; }
  .tag-input {
    width: 100%;
    background: var(--input-bg);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 3px 6px;
    border-radius: 3px;
    font-size: 12px;
    outline: none;
  }
  .tag-input:focus { border-color: var(--accent); }
  .tag-dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: 4px;
    z-index: 100;
    max-height: 120px;
    overflow-y: auto;
  }
  .tag-opt {
    padding: 4px 8px;
    cursor: pointer;
    font-size: 12px;
  }
  .tag-opt:hover { background: var(--hover); }

  /* Timing */
  .quick-dates { display: flex; gap: 6px; margin-bottom: 8px; }
  .quick-btn {
    background: var(--hover-btn);
    border: 1px solid var(--border);
    color: var(--text-dim);
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 3px;
    cursor: pointer;
  }
  .quick-btn:hover { color: var(--accent); border-color: var(--accent); }

  .date-row {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 4px;
    font-size: 12px;
  }
  .date-input, .time-input {
    background: var(--input-bg);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 2px 4px;
    border-radius: 3px;
    font-size: 12px;
    outline: none;
  }
  .date-input:focus, .time-input:focus { border-color: var(--accent); }
  .date-input { width: 130px; }
  .time-input { width: 75px; }
  .time-add {
    background: none;
    border: none;
    color: var(--accent);
    font-size: 11px;
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 3px;
    opacity: 0.7;
  }
  .time-add:hover { opacity: 1; background: var(--hover); }
  .time-clear {
    background: none;
    border: none;
    color: var(--text-dim);
    font-size: 11px;
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 3px;
  }
  .time-clear:hover { color: var(--red); }

  .recurrence-btn {
    margin-top: 8px;
    background: none;
    border: 1px solid var(--border);
    color: var(--text-dim);
    font-size: 12px;
    padding: 3px 8px;
    border-radius: 3px;
    cursor: pointer;
  }
  .recurrence-btn:hover { color: var(--accent); border-color: var(--accent); }

  .recurrence-summary {
    font-size: 11px;
    color: var(--accent);
    margin-top: 3px;
    padding-left: 2px;
  }

  /* Format */
  .fmt-row {
    display: flex;
    gap: 4px;
    margin-bottom: 4px;
  }
  .fmt-row.disabled { opacity: 0.4; pointer-events: none; }
  .fmt-btn {
    background: var(--hover-btn);
    border: 1px solid var(--border);
    width: 26px;
    height: 26px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 3px;
    cursor: pointer;
    font-size: 13px;
    user-select: none;
  }
  .fmt-btn.active { background: var(--accent-dim); border-color: var(--accent); }
  .fmt-btn:hover { border-color: var(--accent); }
  .fmt-color {
    background: var(--hover-btn);
    border: 1px solid var(--border);
    width: 26px;
    height: 26px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 3px;
    cursor: pointer;
    font-size: 11px;
    position: relative;
  }
</style>
