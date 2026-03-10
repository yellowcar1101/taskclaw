<script lang="ts">
  import { showRapidInput, flags, tags, allTasks, createTask, navigateToOutline } from '../stores/tasks';
  import { parseRapidInput, formatDateDisplay } from '../parsing';
  import type { Task } from '../types';
  import { get } from 'svelte/store';

  let rawText = '';
  let applyParsing = true;
  let parentId: string | null = null;
  let parentSearch = '';
  let importing = false;
  let error = '';

  $: parsedLines = parseRapidInput(rawText, $flags, $tags, applyParsing);

  // Parent picker
  $: filteredTasks = parentSearch
    ? $allTasks.filter(t => t.caption.toLowerCase().includes(parentSearch.toLowerCase())).slice(0, 20)
    : $allTasks.slice(0, 20);

  let showParentDropdown = false;
  $: parentTask = parentId ? $allTasks.find(t => t.id === parentId) : null;
  $: parentLabel = parentTask ? parentTask.caption : 'Root (no parent)';

  function close() {
    showRapidInput.set(false);
    rawText = '';
    parentId = null;
    parentSearch = '';
    error = '';
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
  }

  async function doImport() {
    if (!parsedLines.length) return;
    importing = true;
    error = '';
    try {
      // Build parent stack
      const idStack: (string | null)[] = [parentId];
      const depthStack: number[] = [-1];
      let firstId: string | null = null;

      for (const line of parsedLines) {
        // Find parent
        while (depthStack.length > 1 && depthStack[depthStack.length - 1] >= line.depth) {
          idStack.pop();
          depthStack.pop();
        }
        const pid = idStack[idStack.length - 1];

        const task = await createTask({
          parent_id: pid,
          caption: line.parsed.caption,
          note: '',
          flag_id: line.parsed.flagId ?? undefined,
          starred: line.parsed.starred,
          start_date: line.parsed.startDate ?? undefined,
          due_date: line.parsed.dueDate ?? undefined,
          reminder_at: line.parsed.reminderAt ?? undefined,
          tag_ids: line.parsed.tagIds,
        });
        if (!firstId) firstId = task.id;
        idStack.push(task.id);
        depthStack.push(line.depth);
      }

      close();
      if (firstId) navigateToOutline(firstId);
    } catch (e: any) {
      error = e?.message ?? String(e);
    } finally {
      importing = false;
    }
  }

  function onGlobalKeydown(e: KeyboardEvent) {
    if (e.ctrlKey && e.key === 'Enter') doImport();
  }
</script>

<svelte:window on:keydown={onGlobalKeydown} />

<div class="overlay" on:click|self={close} role="dialog" aria-modal="true">
  <div class="modal" on:keydown={onKeydown} role="document">
    <!-- Header -->
    <div class="modal-header">
      <span class="modal-title">📋 Rapid Input</span>
      <button class="close-btn" on:click={close}>✕</button>
    </div>

    <!-- Options bar -->
    <div class="options-bar">
      <div style="position:relative;flex:1">
        <button class="parent-btn" on:click={() => showParentDropdown = !showParentDropdown}>
          Parent: {parentLabel} ▾
        </button>
        {#if showParentDropdown}
          <div class="parent-dropdown">
            <input class="parent-search" bind:value={parentSearch} placeholder="Search tasks…" autofocus />
            <div class="parent-opt" on:click={() => { parentId = null; showParentDropdown = false; }} role="option" tabindex="0" on:keydown>
              Root (no parent)
            </div>
            {#each filteredTasks as t}
              <div class="parent-opt" on:click={() => { parentId = t.id; showParentDropdown = false; }} role="option" tabindex="0" on:keydown>
                {t.caption}
              </div>
            {/each}
          </div>
        {/if}
      </div>
      <label class="chk-row">
        <input type="checkbox" bind:checked={applyParsing} />
        Apply parsing
      </label>
    </div>

    <!-- Split pane -->
    <div class="split">
      <!-- Input -->
      <textarea
        class="rapid-input"
        bind:value={rawText}
        placeholder="Paste tasks here, one per line.&#10;Indent with tabs or spaces for subtasks.&#10;&#10;Example:&#10;Project Alpha d:next monday&#10;  Task 1 !Urgent&#10;  Task 2 #dev"
      ></textarea>

      <!-- Preview -->
      <div class="preview">
        {#if parsedLines.length === 0}
          <div class="preview-empty">Preview will appear here</div>
        {:else}
          {#each parsedLines as line}
            <div class="preview-line" style="padding-left:{line.depth * 16 + 4}px">
              <span class="prev-bullet">{'└─'}</span>
              <span class="prev-caption">{line.parsed.caption}</span>
              {#if line.parsed.starred}<span class="prev-badge star">⭐</span>{/if}
              {#if line.parsed.flagId}
                {@const f = $flags.find(x => x.id === line.parsed.flagId)}
                {#if f}<span class="prev-dot" style="background:{f.color}"></span>{/if}
              {/if}
              {#if line.parsed.startDate}
                <span class="prev-badge green">s:{formatDateDisplay(line.parsed.startDate)}</span>
              {/if}
              {#if line.parsed.dueDate}
                <span class="prev-badge red">d:{formatDateDisplay(line.parsed.dueDate)}</span>
              {/if}
              {#each line.parsed.tagIds as tid}
                {@const t = $tags.find(x => x.id === tid)}
                {#if t}
                  <span class="prev-badge" style="background:{t.color}22;color:{t.color}">{t.name}</span>
                {/if}
              {/each}
            </div>
          {/each}
        {/if}
      </div>
    </div>

    <!-- Footer -->
    <div class="modal-footer">
      <div class="syntax-hint">
        Syntax: <code>!Flag</code> <code>#tag</code> <code>*</code> (star) <code>s:date</code> <code>d:date</code> <code>remind N min before</code>
      </div>
      <div style="display:flex;gap:8px;align-items:center">
        {#if error}<span class="err">{error}</span>{/if}
        <button class="cancel-btn" on:click={close}>Cancel</button>
        <button class="import-btn" on:click={doImport} disabled={importing || parsedLines.length === 0}>
          {importing ? 'Importing…' : 'Import'} <span class="kbd">Ctrl+↵</span>
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.5);
    z-index: 800;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .modal {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    width: min(900px, 92vw);
    height: 80vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 16px 48px rgba(0,0,0,0.5);
  }

  .modal-header {
    display: flex;
    align-items: center;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .modal-title { font-size: 14px; font-weight: 600; flex: 1; }
  .close-btn { background: none; border: none; color: var(--text-dim); cursor: pointer; font-size: 14px; }
  .close-btn:hover { color: var(--text); }

  .options-bar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 16px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    font-size: 12px;
  }

  .parent-btn {
    background: var(--input-bg);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 3px 10px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
  }
  .parent-btn:hover { border-color: var(--accent); }
  .parent-dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: 4px;
    z-index: 900;
    max-height: 200px;
    overflow-y: auto;
    box-shadow: 0 8px 24px rgba(0,0,0,0.4);
  }
  .parent-search {
    width: 100%;
    background: var(--input-bg);
    border: none;
    border-bottom: 1px solid var(--border);
    color: var(--text);
    padding: 6px 8px;
    font-size: 12px;
    outline: none;
  }
  .parent-opt {
    padding: 5px 12px;
    cursor: pointer;
    font-size: 12px;
  }
  .parent-opt:hover { background: var(--hover); }

  .chk-row {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    font-size: 12px;
    color: var(--text);
  }
  .chk-row input { accent-color: var(--accent); }

  .split {
    flex: 1;
    display: flex;
    overflow: hidden;
  }

  .rapid-input {
    flex: 1;
    background: var(--input-bg);
    border: none;
    border-right: 1px solid var(--border);
    color: var(--text);
    font-family: 'Cascadia Code', monospace;
    font-size: 12px;
    padding: 12px;
    resize: none;
    outline: none;
    line-height: 1.6;
  }

  .preview {
    flex: 1;
    overflow-y: auto;
    padding: 8px 4px;
    background: var(--bg);
  }
  .preview-empty { padding: 20px; color: var(--text-dim); font-size: 12px; }

  .preview-line {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 24px;
    font-size: 12px;
  }
  .prev-bullet { color: var(--text-dim); font-size: 10px; flex-shrink: 0; }
  .prev-caption { color: var(--text); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .prev-badge {
    font-size: 10px;
    padding: 1px 4px;
    border-radius: 8px;
    background: var(--hover-btn);
    color: var(--text-dim);
    flex-shrink: 0;
  }
  .prev-badge.star { background: none; }
  .prev-badge.green { background: #6ABF6922; color: var(--green); }
  .prev-badge.red   { background: #E05C5C22; color: var(--red); }
  .prev-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }

  .modal-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 16px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
    gap: 12px;
  }
  .syntax-hint { font-size: 11px; color: var(--text-dim); }
  .syntax-hint code { background: var(--input-bg); padding: 1px 4px; border-radius: 2px; }
  .err { color: var(--red); font-size: 11px; }
  .cancel-btn {
    background: var(--hover-btn);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 4px 12px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
  }
  .import-btn {
    background: var(--accent);
    border: none;
    color: #fff;
    padding: 4px 14px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .import-btn:disabled { opacity: 0.5; cursor: default; }
  .kbd { font-size: 10px; background: rgba(255,255,255,0.2); padding: 1px 4px; border-radius: 2px; }
</style>
