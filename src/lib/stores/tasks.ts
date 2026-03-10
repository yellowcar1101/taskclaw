import { writable, derived, get } from 'svelte/store';
import type { Task, Flag, Tag, SavedView, SortField, SortDir } from '../types';
import { api } from '../api';

// ── Raw data ──────────────────────────────────────────────────────────────────
export const allTasks = writable<Task[]>([]);
export const flags    = writable<Flag[]>([]);
export const tags     = writable<Tag[]>([]);
export const views    = writable<SavedView[]>([]);

// ── UI state ──────────────────────────────────────────────────────────────────
export const expanded          = writable<Set<string>>(new Set());
export const selected          = writable<Set<string>>(new Set());
export const editingId         = writable<string | null>(null);
export const detailTaskId      = writable<string | null>(null);
export const activeTabId       = writable<string>('outline'); // 'outline' | view.id
export const showPrefs         = writable<boolean>(false);
export const showRapidInput    = writable<boolean>(false);
export const outlineScrollToId = writable<string | null>(null);
export const lastUsedFlagId    = writable<string | null>(null);
export const contextMenu       = writable<{ x: number; y: number; taskId: string } | null>(null);
export const showViewsPanel    = writable<boolean>(false);
export const editingViewId     = writable<string | null>(null); // view settings dialog
export const searchQuery       = writable<string>('');
export const sortField         = writable<SortField>('position');
export const sortDir           = writable<SortDir>('asc');
export const filterFlagId      = writable<string | null>(null);

// ── Derived ───────────────────────────────────────────────────────────────────
export const taskById = derived(allTasks, ts => new Map(ts.map(t => [t.id, t])));

export const childrenOf = derived(allTasks, ts => {
  const map = new Map<string | null, Task[]>();
  for (const t of ts) {
    const key = t.parent_id ?? null;
    if (!map.has(key)) map.set(key, []);
    map.get(key)!.push(t);
  }
  for (const arr of map.values()) arr.sort((a, b) => a.position - b.position);
  return map;
});

export function sortTasks(tasks: Task[], field: SortField, dir: SortDir): Task[] {
  return [...tasks].sort((a, b) => {
    let av: any, bv: any;
    switch (field) {
      case 'caption':    av = a.caption.toLowerCase(); bv = b.caption.toLowerCase(); break;
      case 'due_date':   av = a.due_date ?? '9999';   bv = b.due_date ?? '9999';   break;
      case 'start_date': av = a.start_date ?? '9999'; bv = b.start_date ?? '9999'; break;
      case 'starred':    av = a.starred ? 0 : 1;       bv = b.starred ? 0 : 1;      break;
      default:           av = a.position;              bv = b.position;
    }
    const cmp = av < bv ? -1 : av > bv ? 1 : 0;
    return dir === 'asc' ? cmp : -cmp;
  });
}

export const rootTasks = derived(
  [childrenOf, sortField, sortDir, filterFlagId, searchQuery],
  ([$children, $sf, $sd, $flag, $q]) => {
    let tasks = $children.get(null) ?? [];
    if ($flag) {
      if ($flag === '__starred__') tasks = tasks.filter(t => t.starred);
      else if ($flag === '__today__') {
        const today = new Date().toISOString().slice(0, 10);
        tasks = tasks.filter(t => t.due_date === today);
      } else tasks = tasks.filter(t => t.flag_id === $flag);
    }
    if ($q) {
      const ql = $q.toLowerCase();
      tasks = tasks.filter(t => t.caption.toLowerCase().includes(ql));
    }
    return sortTasks(tasks, $sf, $sd);
  }
);

export function getChildren(parentId: string): Task[] {
  const map = get(childrenOf);
  return map.get(parentId) ?? [];
}

// ── Actions ───────────────────────────────────────────────────────────────────
export async function loadAll() {
  const [tasks, fls, tgs, vws] = await Promise.all([
    api.getAllFlat(), api.getFlags(), api.getTags(), api.getViews()
  ]);
  allTasks.set(tasks);
  flags.set(fls);
  tags.set(tgs);
  views.set(vws);
}

export async function createTask(input: Parameters<typeof api.createTask>[0]) {
  const task = await api.createTask(input);
  allTasks.update(ts => {
    const updated = [...ts, task];
    // Update parent's has_children flag
    if (task.parent_id) {
      return updated.map(t => t.id === task.parent_id ? { ...t, has_children: true } : t);
    }
    return updated;
  });
  return task;
}

export async function updateTask(id: string, input: Parameters<typeof api.updateTask>[1]) {
  const task = await api.updateTask(id, input);
  allTasks.update(ts => ts.map(t => t.id === id ? task : t));
  return task;
}

export async function deleteTask(id: string) {
  const all = get(allTasks);
  const deletedTask = all.find(t => t.id === id);
  const parentId = deletedTask?.parent_id ?? null;

  await api.deleteTask(id);

  const toRemove = new Set<string>();
  function collect(tid: string) {
    toRemove.add(tid);
    all.filter(t => t.parent_id === tid).forEach(c => collect(c.id));
  }
  collect(id);

  allTasks.update(ts => {
    const filtered = ts.filter(t => !toRemove.has(t.id));
    // Update parent's has_children if this was its only child
    if (parentId) {
      const parentStillHasChildren = filtered.some(t => t.parent_id === parentId);
      return filtered.map(t => t.id === parentId ? { ...t, has_children: parentStillHasChildren } : t);
    }
    return filtered;
  });
  detailTaskId.update(did => toRemove.has(did ?? '') ? null : did);
}

export async function completeTask(id: string, completed: boolean) {
  const task = await api.completeTask(id, completed);
  const parentId = get(taskById).get(id)?.parent_id ?? null;
  if (completed) {
    allTasks.update(ts => {
      const filtered = ts.filter(t => t.id !== id);
      if (parentId) {
        const parentStillHasChildren = filtered.some(t => t.parent_id === parentId);
        return filtered.map(t => t.id === parentId ? { ...t, has_children: parentStillHasChildren } : t);
      }
      return filtered;
    });
    detailTaskId.update(did => did === id ? null : did);
  } else {
    allTasks.update(ts => ts.map(t => t.id === id ? task : t));
  }
}

export async function moveTask(id: string, newParentId: string | null, newPosition: number) {
  const oldParentId = get(taskById).get(id)?.parent_id ?? null;
  const task = await api.moveTask(id, newParentId, newPosition);
  allTasks.update(ts => {
    const mapped = ts.map(t => t.id === id ? task : t);
    // Old parent might have lost its last child
    if (oldParentId) {
      const stillHas = mapped.some(t => t.parent_id === oldParentId);
      return mapped.map(t => t.id === oldParentId ? { ...t, has_children: stillHas } : t);
    }
    // New parent gained a child
    if (newParentId) {
      return mapped.map(t => t.id === newParentId ? { ...t, has_children: true } : t);
    }
    return mapped;
  });
}

export async function reorderTasks(idsAndPositions: [string, number][]) {
  await api.reorderTasks(idsAndPositions);
  allTasks.update(ts => ts.map(t => {
    const entry = idsAndPositions.find(([id]) => id === t.id);
    return entry ? { ...t, position: entry[1] } : t;
  }));
}

// ── UI helpers ─────────────────────────────────────────────────────────────────
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

export function setSelected(id: string, multi: boolean) {
  selected.update(s => {
    if (multi) {
      const next = new Set(s);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    }
    return new Set([id]);
  });
  if (!multi) detailTaskId.set(id);
}

export function clearSelection() {
  selected.set(new Set());
  detailTaskId.set(null);
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

export function navigateToOutline(id: string) {
  activeTabId.set('outline');
  const map = get(taskById);
  let t = map.get(id);
  while (t?.parent_id) {
    expanded.update(s => { const n = new Set(s); n.add(t!.parent_id!); return n; });
    t = map.get(t.parent_id);
  }
  outlineScrollToId.set(id);
  setSelected(id, false);
}

export function expandToTask(id: string) {
  navigateToOutline(id);
}
