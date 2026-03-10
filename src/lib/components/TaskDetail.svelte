<script lang="ts">
  import { detailTaskId, taskById, flags, tags, updateTask, deleteTask, closeDetail } from '../stores/tasks';
  import { api } from '../api';

  $: task = $detailTaskId ? $taskById.get($detailTaskId) : null;

  let caption = '';
  let note = '';
  let startDate = '';
  let dueDate = '';
  let flagId = '';
  let starred = false;
  let selectedTagIds: string[] = [];
  let gmailLink = '';
  let outlookLink = '';
  let saving = false;

  $: if (task) {
    caption = task.caption;
    note = task.note;
    startDate = task.start_date ?? '';
    dueDate = task.due_date ?? '';
    flagId = task.flag_id ?? '';
    starred = task.starred;
    selectedTagIds = task.tags.map(t => t.id);
    gmailLink = '';
    outlookLink = '';
  }

  async function save() {
    if (!task) return;
    saving = true;
    await updateTask(task.id, {
      caption: caption.trim() || task.caption,
      note,
      start_date: startDate || '',
      due_date: dueDate || '',
      flag_id: flagId || '',
      starred,
      tag_ids: selectedTagIds,
    });
    saving = false;
  }

  async function addGmailLink() {
    if (!task || !gmailLink.trim()) return;
    const threadId = gmailLink.trim().replace(/.*\/([^/]+)$/, '$1');
    await api.addEmailLink(task.id, 'gmail', threadId, 'Gmail thread');
    gmailLink = '';
    // reload task
    const updated = await api.updateTask(task.id, {});
    // trigger refresh via store
  }

  async function addOutlookLink() {
    if (!task || !outlookLink.trim()) return;
    await api.addEmailLink(task.id, 'outlook', outlookLink.trim(), 'Outlook email');
    outlookLink = '';
  }

  function insertMd(prefix: string, suffix = '') {
    const el = document.getElementById('note-textarea') as HTMLTextAreaElement;
    if (!el) return;
    const start = el.selectionStart;
    const end = el.selectionEnd;
    const sel = note.slice(start, end) || 'text';
    note = note.slice(0, start) + prefix + sel + suffix + note.slice(end);
    setTimeout(() => {
      el.focus();
      el.setSelectionRange(start + prefix.length, start + prefix.length + sel.length);
    }, 10);
  }

  function close() { save(); closeDetail(); }
</script>

{#if task}
  <div class="detail-panel">
    <div class="detail-header">
      <span class="detail-title">Task</span>
      <div class="detail-actions">
        <button class="icon-btn star" class:active={starred} on:click={() => { starred = !starred; save(); }}
          title="Star">★</button>
        <button class="icon-btn close-btn" on:click={close} title="Close">✕</button>
      </div>
    </div>

    <div class="detail-body">
      <!-- Caption -->
      <input class="caption-field" bind:value={caption} on:blur={save} placeholder="Task name" />

      <!-- Flag -->
      <div class="field-row">
        <label>Flag</label>
        <select bind:value={flagId} on:change={save}>
          <option value="">— none —</option>
          {#each $flags as f}
            <option value={f.id}>
              {f.name}
            </option>
          {/each}
        </select>
      </div>

      <!-- Dates -->
      <div class="field-row">
        <label>Start</label>
        <input type="date" bind:value={startDate} on:change={save} />
      </div>
      <div class="field-row">
        <label>Due</label>
        <input type="date" bind:value={dueDate} on:change={save} />
      </div>

      <!-- Tags -->
      <div class="field-row tags-row">
        <label>Tags</label>
        <div class="tag-list">
          {#each $tags as tag}
            <label class="tag-toggle" class:active={selectedTagIds.includes(tag.id)}>
              <input type="checkbox" checked={selectedTagIds.includes(tag.id)}
                on:change={e => {
                  if ((e.target as HTMLInputElement).checked) selectedTagIds = [...selectedTagIds, tag.id];
                  else selectedTagIds = selectedTagIds.filter(id => id !== tag.id);
                  save();
                }} />
              <span style="background:{tag.color}22;color:{tag.color};border-color:{tag.color}55">{tag.name}</span>
            </label>
          {/each}
        </div>
      </div>

      <!-- Notes -->
      <div class="note-section">
        <div class="note-toolbar">
          <button class="md-btn" on:click={() => insertMd('**', '**')} title="Bold">B</button>
          <button class="md-btn italic" on:click={() => insertMd('_', '_')} title="Italic">I</button>
          <button class="md-btn" on:click={() => insertMd('- ')} title="List">•</button>
          <button class="md-btn" on:click={() => insertMd('# ')} title="Heading">H</button>
          <button class="md-btn" on:click={() => insertMd('[', '](url)')} title="Link">🔗</button>
          <button class="md-btn" on:click={() => insertMd('![alt](', ')')} title="Image">🖼</button>
          <button class="md-btn" on:click={() => insertMd('`', '`')} title="Code">`</button>
        </div>
        <textarea
          id="note-textarea"
          bind:value={note}
          on:blur={save}
          placeholder="Notes (Markdown supported)…"
          rows="8"
        ></textarea>
      </div>

      <!-- Email links -->
      <div class="section-label">Email Links</div>
      {#each task.email_links as link}
        <div class="email-link-row">
          <span class="link-icon">{link.link_type === 'gmail' ? '✉' : '📧'}</span>
          <a href={link.link_type === 'gmail'
            ? `https://mail.google.com/mail/u/0/#search/${link.link_data}`
            : link.link_data}
            target="_blank" class="link-text">{link.subject ?? link.link_data}</a>
          <button class="icon-btn danger-sm" on:click={() => api.deleteEmailLink(link.id)} title="Remove">✕</button>
        </div>
      {/each}
      <div class="add-link-row">
        <input class="link-input" bind:value={gmailLink} placeholder="Gmail thread URL…"
          on:keydown={e => e.key === 'Enter' && addGmailLink()} />
        <button class="add-link-btn" on:click={addGmailLink}>+ Gmail</button>
      </div>
      <div class="add-link-row">
        <input class="link-input" bind:value={outlookLink} placeholder="[Outlook:EntryID]…"
          on:keydown={e => e.key === 'Enter' && addOutlookLink()} />
        <button class="add-link-btn" on:click={addOutlookLink}>+ Outlook</button>
      </div>

      <!-- Meta -->
      <div class="meta-row">
        <span>Created: {new Date(task.created_at).toLocaleDateString('en-GB')}</span>
        <span>Modified: {new Date(task.updated_at).toLocaleDateString('en-GB')}</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .detail-panel {
    width: 280px; flex-shrink: 0;
    background: var(--surface);
    border-left: 1px solid var(--border);
    display: flex; flex-direction: column;
    overflow: hidden;
  }

  .detail-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 6px 10px; border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .detail-title { font-size: 11px; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-dim); font-family: sans-serif; }
  .detail-actions { display: flex; gap: 4px; }

  .detail-body { flex: 1; overflow-y: auto; padding: 10px; display: flex; flex-direction: column; gap: 8px; }

  .caption-field {
    background: transparent; border: none; border-bottom: 1px solid var(--border);
    color: var(--text); font-family: 'Cascadia Code', 'Fira Code', monospace;
    font-size: 13px; padding: 2px 0; outline: none; width: 100%;
  }
  .caption-field:focus { border-bottom-color: var(--accent); }

  .field-row { display: flex; align-items: center; gap: 8px; }
  .field-row label { font-size: 11px; color: var(--text-dim); width: 44px; flex-shrink: 0; font-family: sans-serif; }

  select, input[type="date"] {
    background: var(--input-bg); border: 1px solid var(--border);
    color: var(--text); padding: 3px 6px; border-radius: 3px;
    font-size: 12px; outline: none; flex: 1;
  }
  select:focus, input[type="date"]:focus { border-color: var(--accent); }

  .tags-row { align-items: flex-start; }
  .tag-list { display: flex; flex-wrap: wrap; gap: 4px; flex: 1; }
  .tag-toggle input { display: none; }
  .tag-toggle span {
    font-size: 10px; padding: 2px 7px; border-radius: 10px; border: 1px solid;
    cursor: pointer; display: block; opacity: 0.5; transition: opacity 0.1s;
    font-family: sans-serif;
  }
  .tag-toggle.active span { opacity: 1; }

  .note-section { display: flex; flex-direction: column; gap: 4px; }
  .note-toolbar {
    display: flex; gap: 2px; padding: 3px 4px;
    background: var(--surface-elevated); border: 1px solid var(--border);
    border-bottom: none; border-radius: 4px 4px 0 0;
  }
  .md-btn {
    background: none; border: 1px solid transparent; color: var(--text-dim);
    padding: 2px 6px; border-radius: 3px; cursor: pointer; font-size: 12px;
    font-family: sans-serif; transition: background 0.1s, color 0.1s;
  }
  .md-btn:hover { background: var(--hover); color: var(--text); }
  .md-btn.italic { font-style: italic; }

  textarea {
    background: var(--input-bg); border: 1px solid var(--border);
    color: var(--text); padding: 6px 8px; border-radius: 0 0 4px 4px;
    font-size: 12px; font-family: 'Cascadia Code', 'Fira Code', monospace;
    resize: vertical; outline: none; width: 100%; line-height: 1.5;
  }
  textarea:focus { border-color: var(--accent); }

  .section-label { font-size: 10px; color: var(--text-dim); text-transform: uppercase; letter-spacing: 0.06em; font-family: sans-serif; margin-top: 4px; }

  .email-link-row { display: flex; align-items: center; gap: 6px; padding: 2px 0; }
  .link-icon { font-size: 12px; flex-shrink: 0; }
  .link-text { font-size: 11px; color: var(--accent); flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; text-decoration: none; font-family: sans-serif; }
  .link-text:hover { text-decoration: underline; }
  .danger-sm { background: none; border: none; color: var(--text-dim); cursor: pointer; font-size: 10px; padding: 1px 3px; }
  .danger-sm:hover { color: var(--red); }

  .add-link-row { display: flex; gap: 4px; }
  .link-input { flex: 1; background: var(--input-bg); border: 1px solid var(--border); color: var(--text); padding: 3px 6px; border-radius: 3px; font-size: 11px; outline: none; }
  .link-input:focus { border-color: var(--accent); }
  .add-link-btn { background: var(--hover-btn); border: 1px solid var(--border); color: var(--text-dim); padding: 3px 8px; border-radius: 3px; cursor: pointer; font-size: 11px; white-space: nowrap; }
  .add-link-btn:hover { color: var(--text); }

  .meta-row { display: flex; gap: 12px; font-size: 10px; color: var(--text-dim); font-family: sans-serif; margin-top: 4px; }

  .icon-btn { background: none; border: none; cursor: pointer; padding: 2px 5px; border-radius: 3px; font-size: 13px; color: var(--text-dim); transition: color 0.1s; }
  .icon-btn.star { color: var(--text-dim); }
  .icon-btn.star.active { color: var(--gold); }
  .icon-btn.close-btn:hover { color: var(--red); }
</style>
