/**
 * TaskClaw Plan View filter — Active / Available / Next Actions algorithm
 */

import type { Task, SavedView } from './types';

export type ActionFilter = 'all' | 'active' | 'available' | 'next_actions';

export interface FilterOptions {
  actionFilter: ActionFilter;
  showCompleted: boolean;
  searchQuery: string;
  flagId: string | null;
  starred?: boolean;
}

// ── Public API ────────────────────────────────────────────────────────────────

export function filterTasksForView(allTasks: Task[], opts: FilterOptions): Task[] {
  const today = todayStr();
  const taskMap = new Map<string, Task>(allTasks.map(t => [t.id, t]));

  // childrenMap: parent_id → sorted children (by position)
  const childrenMap = new Map<string | null, Task[]>();
  for (const t of allTasks) {
    const k = t.parent_id ?? null;
    if (!childrenMap.has(k)) childrenMap.set(k, []);
    childrenMap.get(k)!.push(t);
  }
  for (const arr of childrenMap.values()) arr.sort((a, b) => a.position - b.position);

  // Cache results for ancestor walks
  const ancestorCompletedCache = new Map<string, boolean>();
  const ancestorHiddenCache    = new Map<string, boolean>();

  function ancestorCompleted(t: Task): boolean {
    if (!t.parent_id) return false;
    if (ancestorCompletedCache.has(t.id)) return ancestorCompletedCache.get(t.id)!;
    const parent = taskMap.get(t.parent_id);
    const result = !!parent && (!!parent.completed_at || ancestorCompleted(parent));
    ancestorCompletedCache.set(t.id, result);
    return result;
  }

  function ancestorHidden(t: Task): boolean {
    if (!t.parent_id) return false;
    if (ancestorHiddenCache.has(t.id)) return ancestorHiddenCache.get(t.id)!;
    const parent = taskMap.get(t.parent_id);
    const result = !!parent && (parent.hide_in_views || ancestorHidden(parent));
    ancestorHiddenCache.set(t.id, result);
    return result;
  }

  // Criterion 6: blocked by subtasks_in_order at any ancestor level
  function isBlockedByOrder(t: Task): boolean {
    let current: Task | undefined = t;
    while (current?.parent_id) {
      const parent = taskMap.get(current.parent_id);
      if (!parent) break;
      if (parent.subtasks_in_order) {
        const siblings = childrenMap.get(parent.id) ?? [];
        const myIdx = siblings.findIndex(s => s.id === current!.id);
        for (let i = 0; i < myIdx; i++) {
          const sib = siblings[i];
          // A sibling is "not blocking" if it's completed, folder, or hidden
          if (!sib.completed_at && !sib.is_folder && !sib.hide_in_views) return true;
        }
      }
      current = parent;
    }
    return false;
  }

  function passesBaseFilters(t: Task): boolean {
    if (!opts.showCompleted && t.completed_at) return false;
    if (opts.actionFilter !== 'all' && t.completed_at) return false;
    if (opts.starred && !t.starred) return false;
    if (opts.flagId) {
      if (opts.flagId === '__starred__' && !t.starred) return false;
      if (opts.flagId === '__today__' && t.due_date !== today) return false;
      if (opts.flagId !== '__starred__' && opts.flagId !== '__today__' && t.flag_id !== opts.flagId) return false;
    }
    if (opts.searchQuery) {
      const ql = opts.searchQuery.toLowerCase();
      if (!t.caption.toLowerCase().includes(ql)) return false;
    }
    return true;
  }

  if (opts.actionFilter === 'all') {
    return allTasks.filter(passesBaseFilters);
  }

  // Active criteria (1–7, skip 6 for "available")
  const activeCandidates: Task[] = [];
  for (const t of allTasks) {
    if (!passesBaseFilters(t)) continue;
    if (t.completed_at) continue;                     // 1. not completed
    if (t.has_children) continue;                      // 1. no open subtasks (leaf)
    if (t.is_folder) continue;                         // 4. not folder
    if (t.hide_in_views) continue;                     // 5. not hidden
    if (t.start_date && t.start_date > today) continue; // 3. start date ok
    if (ancestorCompleted(t)) continue;                // 2. no completed parents
    if (ancestorHidden(t)) continue;                   // 5. no hidden parents
    activeCandidates.push(t);
  }

  if (opts.actionFilter === 'available') {
    return activeCandidates; // skip criterion 6
  }

  // Active: also apply criterion 6
  const active = activeCandidates.filter(t => !isBlockedByOrder(t));

  if (opts.actionFilter === 'active') {
    return active;
  }

  // Next Actions: first active task per project root
  // A "project" is: a task with is_project=true or a root task (parent_id=null)
  // For each active task, find its nearest project ancestor (or the root task itself)
  const activeSet = new Set(active.map(t => t.id));
  const projectFirst = new Map<string, Task>(); // projectId → first active task

  // Sort by position to ensure "first" is deterministic
  const sorted = [...active].sort((a, b) => a.position - b.position);

  for (const t of sorted) {
    // Walk up to find project or root
    let projectId: string = t.id;
    let cur: Task | undefined = t;
    while (cur) {
      if (cur.is_project || !cur.parent_id) {
        projectId = cur.id;
        break;
      }
      cur = cur.parent_id ? taskMap.get(cur.parent_id) : undefined;
    }
    if (!projectFirst.has(projectId)) {
      projectFirst.set(projectId, t);
    }
  }

  return Array.from(projectFirst.values());
}

// ── Grouping ──────────────────────────────────────────────────────────────────

export interface TaskGroup {
  key: string;
  label: string;
  color?: string;
  tasks: Task[];
}

export function groupTasks(tasks: Task[], groupBy: string): TaskGroup[] {
  if (!groupBy || groupBy === 'none') {
    return [{ key: '__all__', label: '', tasks }];
  }

  const today = todayStr();
  const thisWeek = addDays(today, 7);

  if (groupBy === 'tag') {
    // Special: task can appear in multiple groups
    const tagGroups = new Map<string, { label: string; color: string; tasks: Task[] }>();
    const noTagTasks: Task[] = [];
    for (const t of tasks) {
      if (!t.tags.length) { noTagTasks.push(t); continue; }
      for (const tag of t.tags) {
        if (!tagGroups.has(tag.id)) tagGroups.set(tag.id, { label: tag.name, color: tag.color, tasks: [] });
        tagGroups.get(tag.id)!.tasks.push(t);
      }
    }
    const result: TaskGroup[] = Array.from(tagGroups.entries()).map(([key, g]) => ({
      key, label: g.label, color: g.color, tasks: g.tasks,
    }));
    if (noTagTasks.length) result.push({ key: '__notag__', label: 'No Tag', tasks: noTagTasks });
    return result;
  }

  const groups = new Map<string, TaskGroup>();

  function getGroup(key: string, label: string, color?: string): TaskGroup {
    if (!groups.has(key)) groups.set(key, { key, label, color, tasks: [] });
    return groups.get(key)!;
  }

  for (const t of tasks) {
    switch (groupBy) {
      case 'flag': {
        const key = t.flag?.id ?? '__noflag__';
        const label = t.flag?.name ?? 'No Flag';
        const color = t.flag?.color;
        getGroup(key, label, color).tasks.push(t);
        break;
      }
      case 'due_date': {
        let key: string, label: string;
        if (!t.due_date)              { key = '__nodue__';   label = 'No Due Date'; }
        else if (t.due_date < today)  { key = '__overdue__'; label = 'Overdue'; }
        else if (t.due_date === today){ key = '__today__';   label = 'Today'; }
        else if (t.due_date <= thisWeek){ key = '__week__';  label = 'This Week'; }
        else                          { key = '__later__';   label = 'Later'; }
        getGroup(key, label).tasks.push(t);
        break;
      }
      case 'start_date': {
        const key = t.start_date ?? '__nostart__';
        const label = t.start_date ?? 'No Start Date';
        getGroup(key, label).tasks.push(t);
        break;
      }
      default:
        getGroup('__all__', '').tasks.push(t);
    }
  }

  // Order due_date buckets properly
  if (groupBy === 'due_date') {
    const order = ['__overdue__', '__today__', '__week__', '__later__', '__nodue__'];
    return order.filter(k => groups.has(k)).map(k => groups.get(k)!);
  }

  return Array.from(groups.values());
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function todayStr(): string {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}`;
}

function addDays(isoDate: string, n: number): string {
  const d = new Date(isoDate + 'T00:00:00');
  d.setDate(d.getDate() + n);
  return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}`;
}
