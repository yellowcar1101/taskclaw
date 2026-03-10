import { writable, derived, get } from 'svelte/store';
import type { Task, Flag, Tag, SavedView, TaskGroup, GroupByField, SortByField } from '../types';
import { api } from '../api';

// ── Raw data ──────────────────────────────────────────────────────────────────
export const allTasks   = writable<Task[]>([]);
export const flags      = writable<Flag[]>([]);
export const tags       = writable<Tag[]>([]);
export const views      = writable<SavedView[]>([]);

// ── UI state ──────────────────────────────────────────────────────────────────
export const expanded    = writable<Set<string>>(new Set());
export const selected    = writable<Set<string>>(new Set());
export const editingId   = writable<string | null>(null);
export const detailTaskId= writable<string | null>(null);
export const activeTabId = writable<string>('outline'); // 'outline' | view.id
export const rightPanelOpen = writable<boolean>(true);
export const searchQuery = writable<string>('');
export const collapsedGroups = writable<Set<string>>(new Set());
export const showPrefs   = writable<boolean>(false);

// ── Derived ───────────────────────────────────────────────────────────────────
export const taskMap = derived(allTasks, ($tasks) => {
  const map = new Map<string | null, Task[]>();
  for (const t of $tasks) {
    const key = t.parent_id ?? null;
    if (!map.has(key)) map.set(key, []);
    map.get(key)!.push(t);
  }
  return map;
});

export const taskById = derived(allTasks, ($tasks) => {
  const map = new Map<string, Task>();
  for (const t of $tasks) map.set(t.id, t);
  return map;
});

export function getChildren(parentId: string): Task[] {
  const map = get(taskMap);
  return [...(map.get(parentId) ?? [])].sort((a, b) => a.position - b.position);
}

export const rootTasks = derived([taskMap, searchQuery], ([$map, $q]) => {
  let tasks = [...($map.get(null) ?? [])].sort((a, b) => a.position - b.position);
  if ($q) tasks = tasks.filter(t => t.caption.toLowerCase().includes($q.toLowerCase()));
  return tasks;
});

// ── View grouping ─────────────────────────────────────────────────────────────
function dueBucket(task: Task): { key: string; label: string; order: number } {
  const due = task.due_date;
  if (!due) return { key: 'no_date', label: 'No Date', order: 99 };
  const today = new Date(); today.setHours(0,0,0,0);
  const d = new Date(due + 'T00:00:00');
  const diff = Math.floor((d.getTime() - today.getTime()) / 86400000);
  if (diff < 0)  return { key: 'overdue',   label: 'Overdue',    order: 0 };
  if (diff === 0) return { key: 'today',    label: 'Today',      order: 1 };
  if (diff === 1) return { key: 'tomorrow', label: 'Tomorrow',   order: 2 };
  if (diff <= 7)  return { key: 'week',     label: 'This Week',  order: 3 };
  return              { key: 'later',    label: 'Later',      order: 4 };
}

function startBucket(task: Task): { key: string; label: string; order: number } {
  const start = task.start_date;
  if (!start) return { key: 'no_start', label: 'No Start Date', order: 99 };
  const today = new Date(); today.setHours(0,0,0,0);
  const d = new Date(start + 'T00:00:00');
  const diff = Math.floor((d.getTime() - today.getTime()) / 86400000);
  if (diff <= 0) return { key: 'active',  label: 'Active',   order: 0 };
  if (diff <= 7) return { key: 'soon',    label: 'Starting Soon', order: 1 };
  return             { key: 'future',  label: 'Future',   order: 2 };
}

function dateBucket(dateStr: string | null, label: string): { key: string; label: string; order: number } {
  if (!dateStr) return { key: 'none', label: 'Unknown', order: 99 };
  const d = new Date(dateStr);
  const key = d.toISOString().slice(0, 7); // YYYY-MM
  const l = d.toLocaleDateString('en-GB', { month: 'long', year: 'numeric' });
  return { key, label: `${label}: ${l}`, order: d.getTime() };
}

export function groupTasks(tasks: Task[], groupBy: GroupByField, allFlags: Flag[]): TaskGroup[] {
  if (groupBy === 'none') {
    return [{ key: 'all', label: '', color: undefined, tasks, collapsed: false }];
  }

  const buckets = new Map<string, TaskGroup>();

  for (const task of tasks) {
    let key: string, label: string, color: string | undefined, order = 0;

    if (groupBy === 'flag') {
      if (task.flag) {
        key = task.flag.id; label = task.flag.name; color = task.flag.color; order = task.flag.position;
      } else {
        key = '__no_flag__'; label = 'No Flag'; color = undefined; order = 9999;
      }
    } else if (groupBy === 'tag') {
      const firstTag = task.tags[0];
      if (firstTag) { key = firstTag.id; label = firstTag.name; color = firstTag.color; }
      else { key = '__no_tag__'; label = 'No Tag'; }
    } else if (groupBy === 'due_date') {
      const b = dueBucket(task); key = b.key; label = b.label; order = b.order;
    } else if (groupBy === 'start_date') {
      const b = startBucket(task); key = b.key; label = b.label; order = b.order;
    } else if (groupBy === 'created_at') {
      const b = dateBucket(task.created_at, 'Created'); key = b.key; label = b.label; order = b.order;
    } else { // updated_at
      const b = dateBucket(task.updated_at, 'Modified'); key = b.key; label = b.label; order = b.order;
    }

    if (!buckets.has(key)) {
      const collapsed = get(collapsedGroups).has(key);
      buckets.set(key, { key, label, color, tasks: [], collapsed });
    }
    buckets.get(key)!.tasks.push(task);
  }

  return [...buckets.values()].sort((a, b) => {
    // Keep original insertion order for flag groups (sorted by flag.position)
    return 0;
  });
}

export function sortTasks(tasks: Task[], sortBy: SortByField, sortDir: 'asc' | 'desc'): Task[] {
  return [...tasks].sort((a, b) => {
    let av: any, bv: any;
    switch (sortBy) {
      case 'caption':    av = a.caption.toLowerCase();  bv = b.caption.toLowerCase(); break;
      case 'start_date': av = a.start_date ?? '9999';  bv = b.start_date ?? '9999'; break;
      case 'due_date':   av = a.due_date ?? '9999';    bv = b.due_date ?? '9999'; break;
      case 'created_at': av = a.created_at;             bv = b.created_at; break;
      case 'updated_at': av = a.updated_at;             bv = b.updated_at; break;
      case 'flag':       av = a.flag?.position ?? 9999; bv = b.flag?.position ?? 9999; break;
      default:           av = a.position;               bv = b.position;
    }
    const cmp = av < bv ? -1 : av > bv ? 1 : 0;
    return sortDir === 'asc' ? cmp : -cmp;
  });
}

// ── Actions ───────────────────────────────────────────────────────────────────
export async function loadAll() {
  const [tasks, fl, tg, vw] = await Promise.all([
    api.getAllFlat(true), // load all including completed (filtered in views)
    api.getFlags(),
    api.getTags(),
    api.getViews(),
  ]);
  allTasks.set(tasks);
  flags.set(fl);
  tags.set(tg);
  views.set(vw);
}

export async function createTask(input: Parameters<typeof api.createTask>[0]) {
  const task = await api.createTask(input);
  allTasks.update(ts => [...ts, task]);
  return task;
}

export async function updateTask(id: string, input: object) {
  const task = await api.updateTask(id, input);
  allTasks.update(ts => ts.map(t => t.id === id ? task : t));
  return task;
}

export async function deleteTask(id: string) {
  await api.deleteTask(id);
  const all = get(allTasks);
  const toRemove = new Set<string>();
  function collect(tid: string) {
    toRemove.add(tid);
    all.filter(t => t.parent_id === tid).forEach(c => collect(c.id));
  }
  collect(id);
  allTasks.update(ts => ts.filter(t => !toRemove.has(t.id)));
}

export async function completeTask(id: string, completed: boolean) {
  const task = await api.completeTask(id, completed);
  allTasks.update(ts => ts.map(t => t.id === id ? task : t));
}

export async function moveTask(id: string, newParentId: string | null, newPosition: number) {
  const task = await api.moveTask(id, newParentId, newPosition);
  allTasks.update(ts => ts.map(t => t.id === id ? task : t));
}

export async function reorderTasks(idsAndPositions: [string, number][]) {
  await api.reorderTasks(idsAndPositions);
  allTasks.update(ts => ts.map(t => {
    const e = idsAndPositions.find(([id]) => id === t.id);
    return e ? { ...t, position: e[1] } : t;
  }));
}

export function toggleExpanded(id: string) {
  expanded.update(s => { const n = new Set(s); n.has(id) ? n.delete(id) : n.add(id); return n; });
}
export function expandAll() {
  expanded.set(new Set(get(allTasks).filter(t => t.has_children).map(t => t.id)));
}
export function collapseAll() { expanded.set(new Set()); }

export function toggleGroup(key: string) {
  collapsedGroups.update(s => { const n = new Set(s); n.has(key) ? n.delete(key) : n.add(key); return n; });
}

export function setSelected(id: string, multi: boolean) {
  selected.update(s => {
    if (multi) { const n = new Set(s); n.has(id) ? n.delete(id) : n.add(id); return n; }
    return new Set([id]);
  });
}
export function clearSelection() { selected.set(new Set()); }

export function openDetail(id: string) { detailTaskId.set(id); }
export function closeDetail() { detailTaskId.set(null); }
