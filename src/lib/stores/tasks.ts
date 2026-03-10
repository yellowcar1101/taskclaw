import { writable, derived, get } from 'svelte/store';
import type { Task, Context, SortField, SortDir } from '../types';
import { api } from '../api';

// ── Raw data ─────────────────────────────────────────────────────────────────
export const allTasks = writable<Task[]>([]);
export const contexts = writable<Context[]>([]);

// ── UI state ─────────────────────────────────────────────────────────────────
export const expanded = writable<Set<string>>(new Set());
export const selected = writable<Set<string>>(new Set());
export const editingId = writable<string | null>(null);
export const sortField = writable<SortField>('position');
export const sortDir = writable<SortDir>('asc');
export const filterContextId = writable<string | null>(null);
export const searchQuery = writable<string>('');

// ── Derived tree ──────────────────────────────────────────────────────────────
export const taskMap = derived(allTasks, ($tasks) => {
  const map = new Map<string | null, Task[]>();
  for (const t of $tasks) {
    const key = t.parent_id ?? null;
    if (!map.has(key)) map.set(key, []);
    map.get(key)!.push(t);
  }
  return map;
});

function sortTasks(tasks: Task[], field: SortField, dir: SortDir): Task[] {
  return [...tasks].sort((a, b) => {
    let av: any, bv: any;
    switch (field) {
      case 'caption':   av = a.caption.toLowerCase(); bv = b.caption.toLowerCase(); break;
      case 'due_date':  av = a.due_date ?? '9999'; bv = b.due_date ?? '9999'; break;
      case 'score':     av = a.score; bv = b.score; break;
      case 'importance':av = a.importance; bv = b.importance; break;
      case 'urgency':   av = a.urgency; bv = b.urgency; break;
      default:          av = a.position; bv = b.position;
    }
    const cmp = av < bv ? -1 : av > bv ? 1 : 0;
    return dir === 'asc' ? cmp : -cmp;
  });
}

export const rootTasks = derived(
  [taskMap, sortField, sortDir, filterContextId, searchQuery],
  ([$map, $sf, $sd, $ctx, $q]) => {
    let tasks = $map.get(null) ?? [];
    if ($ctx) tasks = tasks.filter(t => t.contexts.some(c => c.id === $ctx));
    if ($q) tasks = tasks.filter(t => t.caption.toLowerCase().includes($q.toLowerCase()));
    return sortTasks(tasks, $sf, $sd);
  }
);

export function getChildren(parentId: string): Task[] {
  const map = get(taskMap);
  const sf = get(sortField);
  const sd = get(sortDir);
  return sortTasks(map.get(parentId) ?? [], sf, sd);
}

// ── Actions ───────────────────────────────────────────────────────────────────
export async function loadAll() {
  const [tasks, ctxs] = await Promise.all([api.getAllFlat(), api.getContexts()]);
  allTasks.set(tasks);
  contexts.set(ctxs);
}

export async function createTask(input: Parameters<typeof api.createTask>[0]) {
  const task = await api.createTask(input);
  allTasks.update(ts => [...ts, task]);
  return task;
}

export async function updateTask(id: string, input: Parameters<typeof api.updateTask>[1]) {
  const task = await api.updateTask(id, input);
  allTasks.update(ts => ts.map(t => t.id === id ? task : t));
  return task;
}

export async function deleteTask(id: string) {
  await api.deleteTask(id);
  // Also remove all descendants from local state
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
  if (completed) {
    allTasks.update(ts => ts.filter(t => t.id !== id));
  } else {
    allTasks.update(ts => ts.map(t => t.id === id ? task : t));
  }
}

export async function moveTask(id: string, newParentId: string | null, newPosition: number) {
  const task = await api.moveTask(id, newParentId, newPosition);
  allTasks.update(ts => ts.map(t => t.id === id ? task : t));
}

export async function reorderTasks(idsAndPositions: [string, number][]) {
  await api.reorderTasks(idsAndPositions);
  allTasks.update(ts => ts.map(t => {
    const entry = idsAndPositions.find(([id]) => id === t.id);
    if (entry) return { ...t, position: entry[1] };
    return t;
  }));
}

export function toggleExpanded(id: string) {
  expanded.update(s => {
    const next = new Set(s);
    if (next.has(id)) next.delete(id); else next.add(id);
    return next;
  });
}

export function expandAll() {
  const all = get(allTasks);
  expanded.set(new Set(all.filter(t => t.has_children).map(t => t.id)));
}

export function collapseAll() {
  expanded.set(new Set());
}

export function setSelected(id: string, multi: boolean, range: boolean) {
  selected.update(s => {
    if (multi) {
      const next = new Set(s);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    }
    return new Set([id]);
  });
}

export function clearSelection() {
  selected.set(new Set());
}

export function toggleSort(field: SortField) {
  sortField.update(sf => {
    if (sf === field) {
      sortDir.update(d => d === 'asc' ? 'desc' : 'asc');
      return sf;
    }
    sortDir.set('asc');
    return field;
  });
}
