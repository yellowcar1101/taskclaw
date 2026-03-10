<script lang="ts">
  import { tick } from 'svelte';
  import {
    allTasks, taskMap, flags, tags, createTask,
    expanded, showRapidInput, activeTabId, navigateToOutline
  } from '../stores/tasks';
  import { parseRapidInput, flattenNodes, type ParsedNode } from '../parsing';
  import type { Task } from '../types';

  let text = '';
  let applyParsing = true;
  let targetTaskId: string | null = null;
  let showParentPicker = false;
  let parentSearch = '';
  let importing = false;
  let textarea: HTMLTextAreaElement;

  $: preview = text.trim() ? parseRapidInput(text, $flags, $tags, applyParsing) : [];
  $: flatPreview = flattenNodes(preview);

  // Build ordered task list for parent picker
  $: treeOrdered = buildTreeOrdered($taskMap);

  function buildTreeOrdered(map: Map<string | null, Task[]>): Array<{ task: Task; depth: number }> {
    const result: Array<{ task: Task; depth: number }> = [];
    function walk(parentId: string | null, depth: number) {
      const children = [...(map.get(parentId) ?? [])].sort((a, b) => a.position - b.position);
      for (const t of children) {
        result.push({ task: t, depth });
        walk(t.id, depth + 1);
      }
    }
    walk(null, 0);
    return result;
  }

  $: filteredTree = parentSearch.trim()
    ? treeOrdered.filter(({ task }) => task.caption.toLowerCase().includes(parentSearch.toLowerCase()))
    : treeOrdered;

  $: selectedTaskName = targetTaskId
    ? ($allTasks.find(t => t.id === targetTaskId)?.caption ?? 'Unknown task')
    : 'Root';

  function countTasks(nodes: ParsedNode[]): number {
    return nodes.reduce((acc, n) => acc + 1 + countTasks(n.children), 0);
  }

  async function doImport() {
    if (!preview.length || importing) return;
    importing = true;

    async function createNodes(nodes: ParsedNode[], parentId: string | null) {
      for (const node of nodes) {
        const { parsed } = node;
        const task = await createTask({
          parent_id: parentId,
          caption: parsed.caption || 'Untitled',
          ...(applyParsing && {
            flag_id:    parsed.flagId    ?? undefined,
            start_date: parsed.startDate ?? undefined,
            due_date:   parsed.dueDate   ?? undefined,
            starred:    parsed.starred   || undefined,
            tag_ids:    parsed.tagIds.length ? parsed.tagIds : undefined,
          }),
        });
        if (node.children.length > 0) await createNodes(node.children, task.id);
      }
    }

    await createNodes(preview, targetTaskId);

    if (targetTaskId) {
      expanded.update(s => { const n = new Set(s); n.add(targetTaskId!); return n; });
      navigateToOutline(targetTaskId);
    } else {
      activeTabId.set('outline');
    }

    text = '';
    importing = false;
    showRapidInput.set(false);
  }

  function close() { showRapidInput.set(false); }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') doImport();
  }

  // Focus textarea when opened
  $: if ($showRapidInput) tick().then(() => textarea?.focus());
</script>

{#if $showRapidInput}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <div class="overlay" on:click|self={close} role="none">
    <div class="dialog" on:keydown={onKeydown} role="none">

      <!-- Header -->
      <div class="dialog-header">
        <span class="dialog-title">Rapid Input</span>
        <div class="header-right">
          <label class="parse-toggle">
            <input type="checkbox" bind:checked={applyParsing} />
            <span>Apply parsing</span>
          </label>
          <button class="close-btn" on:click={close}>✕</button>
        </div>
      </div>

      <div class="dialog-body">
        <!-- Left: input + parent selector -->
        <div class="input-col">

          <!-- Parent task selector -->
          <div class="target-row">
            <span class="target-label">Add to:</span>
            <div class="parent-selector">
              <button class="target-btn" on:click={() => showParentPicker = !showParentPicker}>
                {selectedTaskName}
                <span class="chevron">{showParentPicker ? '▴' : '▾'}</span>
              </button>
              {#if targetTaskId}
                <button class="clear-target" on:click={() => { targetTaskId = null; }}>✕</button>
              {/if}

              {#if showParentPicker}
                <div class="picker-dropdown">
                  <input
                    class="picker-search"
                    bind:value={parentSearch}
                    placeholder="Search tasks…"
                    autofocus
                  />
                  <div class="picker-list">
                    <button
                      class="picker-item"
                      class:selected={targetTaskId === null}
                      on:click={() => { targetTaskId = null; showParentPicker = false; parentSearch = ''; }}
                    >
                      ☰ Root (top level)
                    </button>
                    {#each filteredTree as { task, depth }}
                      <button
                        class="picker-item"
                        class:selected={targetTaskId === task.id}
                        style="padding-left: {12 + depth * 16}px"
                        on:click={() => { targetTaskId = task.id; showParentPicker = false; parentSearch = ''; }}
                      >
                        {#if task.flag}
                          <span class="picker-flag-dot" style="background:{task.flag.color}"></span>
                        {/if}
                        {task.caption}
                      </button>
                    {/each}
                    {#if filteredTree.length === 0}
                      <div class="picker-empty">No tasks found</div>
                    {/if}
                  </div>
                </div>
              {/if}
            </div>
          </div>

          <!-- Text area -->
          <textarea
            bind:this={textarea}
            bind:value={text}
            class="rapid-textarea"
            placeholder="Paste or type tasks — indentation = subtasks.

Natural language (trailing date is parsed automatically):
  Call Jim tomorrow 4pm
  Send report in 3 days remind 10 min in advance
  Organize party May 22 reminder May 21 3pm
  &quot;Buy 3 boxes&quot; next Friday 9am remind me

Explicit tokens:
  !FlagName   #TagName   *   -star   -fl<name>
  s:DATE  d:DATE  r:DATE  -s  -d
  remind / rmd [<when> | N min in advance]

Dates: today  tomorrow  mon  next Fri  in 3d  +2w  Jan26
Ctrl+Enter to import."
            spellcheck="false"
          ></textarea>

          <div class="syntax-ref">
            <code>!Flag</code><code>#tag</code><code>@tag</code><code>*</code><code>-star</code>
            <code>-fl&lt;name&gt;</code><code>-s</code><code>-d</code>
            <span class="sep">·</span>
            <code>remind / rmd</code><code>N min in advance</code>
            <span class="sep">·</span>
            <code>today</code><code>tom</code><code>mon</code><code>next Fri</code>
            <code>in 3d</code><code>in 2w</code><code>Jan26</code><code>5/22</code>
          </div>
        </div>

        <!-- Right: preview -->
        <div class="preview-col">
          <div class="preview-header">
            Preview
            {#if flatPreview.length > 0}
              <span class="preview-count">{flatPreview.length} task{flatPreview.length !== 1 ? 's' : ''}</span>
            {/if}
          </div>

          <div class="preview-body">
            {#if flatPreview.length === 0}
              <div class="preview-empty">Type tasks on the left to preview…</div>
            {:else}
              {#each flatPreview as { node, depth }}
                <div class="preview-row" style="padding-left: {8 + depth * 18}px">
                  <span class="preview-bullet">
                    {node.children.length > 0 ? '▸' : '◦'}
                  </span>

                  <!-- Flag dot -->
                  {#if applyParsing && node.parsed.flagId}
                    {@const flag = $flags.find(f => f.id === node.parsed.flagId)}
                    {#if flag}
                      <span class="pv-flag-dot" style="background:{flag.color}" title={flag.name}></span>
                    {/if}
                  {/if}

                  <!-- Caption -->
                  <span class="pv-caption" class:starred={applyParsing && node.parsed.starred}>
                    {applyParsing ? node.parsed.caption || node.raw : node.raw}
                  </span>

                  <!-- Tags -->
                  {#if applyParsing}
                    {#each node.parsed.tagIds as tagId}
                      {@const tag = $tags.find(t => t.id === tagId)}
                      {#if tag}
                        <span class="pv-tag" style="color:{tag.color};background:{tag.color}22;border-color:{tag.color}55">{tag.name}</span>
                      {/if}
                    {/each}
                  {/if}

                  <!-- Date badges -->
                  {#if applyParsing}
                    {#if node.parsed.startDate}
                      <span class="pv-date start">s:{node.parsed.startDate.slice(5)}</span>
                    {/if}
                    {#if node.parsed.dueDate}
                      <span class="pv-date due">d:{node.parsed.dueDate.slice(5)}</span>
                    {/if}
                    {#if node.parsed.reminderAt}
                      <span class="pv-date rem">r:{node.parsed.reminderAt.slice(5)}</span>
                    {/if}
                  {/if}
                </div>
              {/each}
            {/if}
          </div>
        </div>
      </div>

      <!-- Footer -->
      <div class="dialog-footer">
        <span class="footer-hint">
          {flatPreview.length > 0
            ? `Ready to import ${flatPreview.length} task${flatPreview.length !== 1 ? 's' : ''}${targetTaskId ? ` under "${selectedTaskName}"` : ' at root'}`
            : 'Nothing to import yet'}
        </span>
        <div class="footer-actions">
          <button class="btn-cancel" on:click={close}>Cancel</button>
          <button
            class="btn-import"
            disabled={flatPreview.length === 0 || importing}
            on:click={doImport}
          >
            {importing ? 'Importing…' : `Import (Ctrl+↵)`}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed; inset: 0;
    background: rgba(0,0,0,0.6);
    display: flex; align-items: center; justify-content: center;
    z-index: 900;
  }

  .dialog {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    width: min(900px, 96vw);
    height: min(640px, 90vh);
    display: flex; flex-direction: column;
    overflow: hidden;
    box-shadow: 0 16px 48px rgba(0,0,0,0.6);
  }

  /* Header */
  .dialog-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 10px 16px;
    background: var(--surface-elevated);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .dialog-title { font-size: 14px; font-weight: 600; color: var(--text); font-family: sans-serif; }
  .header-right { display: flex; align-items: center; gap: 12px; }
  .parse-toggle {
    display: flex; align-items: center; gap: 6px;
    font-size: 12px; color: var(--text-dim); cursor: pointer; font-family: sans-serif;
  }
  .parse-toggle input { width: auto; cursor: pointer; }
  .close-btn { background: none; border: none; color: var(--text-dim); cursor: pointer; font-size: 14px; padding: 2px 6px; border-radius: 3px; }
  .close-btn:hover { color: var(--red); }

  /* Body */
  .dialog-body {
    flex: 1; display: flex; gap: 0; overflow: hidden;
  }

  /* Input column */
  .input-col {
    flex: 1; display: flex; flex-direction: column;
    border-right: 1px solid var(--border);
    overflow: hidden;
  }

  .target-row {
    display: flex; align-items: center; gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .target-label { font-size: 11px; color: var(--text-dim); font-family: sans-serif; flex-shrink: 0; }

  .parent-selector { position: relative; flex: 1; }
  .target-btn {
    background: var(--input-bg); border: 1px solid var(--border);
    color: var(--text); padding: 4px 10px; border-radius: 4px;
    cursor: pointer; font-size: 12px; font-family: sans-serif;
    display: flex; align-items: center; gap: 6px;
    max-width: 280px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .target-btn:hover { border-color: var(--accent); }
  .chevron { font-size: 9px; color: var(--text-dim); }
  .clear-target {
    background: none; border: none; color: var(--text-dim); cursor: pointer;
    font-size: 11px; padding: 2px 5px; border-radius: 3px; margin-left: 4px;
  }
  .clear-target:hover { color: var(--red); }

  .picker-dropdown {
    position: absolute; top: calc(100% + 4px); left: 0;
    width: 320px; max-height: 280px;
    background: var(--surface-elevated);
    border: 1px solid var(--border); border-radius: 6px;
    box-shadow: 0 6px 20px rgba(0,0,0,0.4);
    z-index: 100; display: flex; flex-direction: column;
    overflow: hidden;
  }
  .picker-search {
    background: var(--input-bg); border: none; border-bottom: 1px solid var(--border);
    color: var(--text); padding: 7px 10px; font-size: 12px; outline: none;
    flex-shrink: 0;
  }
  .picker-list { overflow-y: auto; flex: 1; }
  .picker-item {
    display: flex; align-items: center; gap: 6px;
    width: 100%; background: none; border: none; color: var(--text-dim);
    padding: 6px 12px; text-align: left; cursor: pointer; font-size: 12px;
    font-family: sans-serif; transition: background 0.06s, color 0.06s;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .picker-item:hover { background: var(--hover); color: var(--text); }
  .picker-item.selected { color: var(--accent); background: var(--selected); }
  .picker-flag-dot { width: 7px; height: 7px; border-radius: 50%; flex-shrink: 0; }
  .picker-empty { font-size: 11px; color: var(--text-dim); padding: 10px 12px; font-family: sans-serif; }

  .rapid-textarea {
    flex: 1; background: var(--input-bg); border: none;
    color: var(--text); padding: 10px 12px;
    font-size: 13px; font-family: 'Cascadia Code', 'Fira Code', 'Consolas', monospace;
    resize: none; outline: none; line-height: 1.6;
    tab-size: 2;
  }
  .rapid-textarea::placeholder { color: var(--text-dim); opacity: 0.7; font-size: 11px; }

  .syntax-ref {
    display: flex; align-items: center; gap: 5px; flex-wrap: wrap;
    padding: 5px 10px; border-top: 1px solid var(--border);
    background: var(--surface-elevated); flex-shrink: 0;
    font-size: 10px; color: var(--text-dim); font-family: sans-serif;
  }
  .syntax-ref code {
    background: var(--hover-btn); border: 1px solid var(--border);
    border-radius: 3px; padding: 0 4px; font-family: monospace; font-size: 10px;
    color: var(--accent);
  }
  .syntax-ref .sep { opacity: 0.4; }

  /* Preview column */
  .preview-col {
    width: 340px; flex-shrink: 0; display: flex; flex-direction: column;
    overflow: hidden;
  }
  .preview-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 8px 12px; border-bottom: 1px solid var(--border);
    font-size: 11px; color: var(--text-dim); font-family: sans-serif;
    text-transform: uppercase; letter-spacing: 0.05em;
    background: var(--surface-elevated); flex-shrink: 0;
  }
  .preview-count {
    background: var(--hover-btn); border: 1px solid var(--border);
    border-radius: 10px; padding: 1px 7px; font-size: 10px;
  }
  .preview-body { flex: 1; overflow-y: auto; padding: 6px 0; }
  .preview-empty { padding: 30px 16px; font-size: 12px; color: var(--text-dim); font-family: sans-serif; text-align: center; }

  .preview-row {
    display: flex; align-items: center; gap: 4px;
    min-height: 26px; padding-right: 8px;
    transition: background 0.04s;
  }
  .preview-row:hover { background: var(--hover); }
  .preview-bullet { font-size: 9px; color: var(--text-dim); flex-shrink: 0; width: 10px; }
  .pv-flag-dot { width: 7px; height: 7px; border-radius: 50%; flex-shrink: 0; }
  .pv-caption {
    font-size: 12px; color: var(--text); flex: 1;
    font-family: 'Cascadia Code', 'Fira Code', monospace;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    min-width: 0;
  }
  .pv-caption.starred::before { content: '★ '; color: var(--gold); }
  .pv-tag { font-size: 10px; padding: 1px 5px; border-radius: 8px; border: 1px solid; white-space: nowrap; flex-shrink: 0; font-family: sans-serif; }
  .pv-date { font-size: 10px; padding: 1px 5px; border-radius: 3px; white-space: nowrap; flex-shrink: 0; font-family: monospace; }
  .pv-date.start { background: #1e3a1e; color: var(--green); }
  .pv-date.due   { background: #3a1e1e; color: var(--red); }
  .pv-date.rem   { background: #2a2a1a; color: var(--gold); }

  /* Footer */
  .dialog-footer {
    display: flex; align-items: center; justify-content: space-between;
    padding: 10px 16px;
    background: var(--surface-elevated);
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }
  .footer-hint { font-size: 11px; color: var(--text-dim); font-family: sans-serif; }
  .footer-actions { display: flex; gap: 8px; }

  .btn-cancel {
    background: var(--hover-btn); border: 1px solid var(--border);
    color: var(--text-dim); padding: 5px 16px; border-radius: 4px;
    cursor: pointer; font-size: 12px; font-family: sans-serif;
  }
  .btn-cancel:hover { color: var(--text); }

  .btn-import {
    background: var(--accent); border: 1px solid var(--accent);
    color: #fff; padding: 5px 20px; border-radius: 4px;
    cursor: pointer; font-size: 12px; font-family: sans-serif; font-weight: 600;
    transition: filter 0.1s;
  }
  .btn-import:hover:not(:disabled) { filter: brightness(1.15); }
  .btn-import:disabled { opacity: 0.4; cursor: default; }
</style>
