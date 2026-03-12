/**
 * Tests for planFilter.ts — filterTasksForView and groupTasks
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { filterTasksForView, groupTasks } from './planFilter';
import type { FilterOptions } from './planFilter';
import type { Task } from './types';

// Fixed reference date: 2026-03-12 (Thursday)
const FIXED_DATE = new Date('2026-03-12T12:00:00.000Z');

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(FIXED_DATE);
});

afterEach(() => {
  vi.useRealTimers();
});

// ── Task factory ──────────────────────────────────────────────────────────────

let _idCounter = 0;
function makeTask(overrides: Partial<Task> = {}): Task {
  const id = `task-${++_idCounter}`;
  return {
    id,
    parent_id: null,
    caption: `Task ${id}`,
    note: '',
    position: _idCounter,
    created_at: '2026-01-01T00:00:00',
    updated_at: '2026-01-01T00:00:00',
    completed_at: null,
    start_date: null,
    due_date: null,
    reminder_at: null,
    recurrence_rule: null,
    flag_id: null,
    flag: null,
    starred: false,
    color: null,
    is_folder: false,
    is_project: false,
    hide_in_views: false,
    subtasks_in_order: false,
    inherit_dates: false,
    custom_format: null,
    tags: [],
    email_links: [],
    has_children: false,
    ...overrides,
  };
}

// Reset counter before each test suite so IDs are predictable
beforeEach(() => { _idCounter = 0; });

const defaultOpts: FilterOptions = {
  actionFilter: 'all',
  showCompleted: true,
  searchQuery: '',
  flagId: null,
};

// ── actionFilter: 'all' ───────────────────────────────────────────────────────

describe("filterTasksForView — actionFilter: 'all'", () => {
  it('returns all tasks when no filters applied', () => {
    const tasks = [makeTask(), makeTask(), makeTask()];
    const result = filterTasksForView(tasks, defaultOpts);
    expect(result).toHaveLength(3);
  });

  it('excludes completed tasks when showCompleted=false', () => {
    const tasks = [
      makeTask({ completed_at: null }),
      makeTask({ completed_at: '2026-03-10T09:00:00' }),
    ];
    const result = filterTasksForView(tasks, { ...defaultOpts, showCompleted: false });
    expect(result).toHaveLength(1);
    expect(result[0].completed_at).toBeNull();
  });

  it('includes completed tasks when showCompleted=true', () => {
    const tasks = [
      makeTask({ completed_at: null }),
      makeTask({ completed_at: '2026-03-10T09:00:00' }),
    ];
    const result = filterTasksForView(tasks, { ...defaultOpts, showCompleted: true });
    expect(result).toHaveLength(2);
  });

  it('starred=true filter: only starred tasks pass', () => {
    const tasks = [
      makeTask({ starred: true }),
      makeTask({ starred: false }),
      makeTask({ starred: true }),
    ];
    const result = filterTasksForView(tasks, { ...defaultOpts, starred: true });
    expect(result).toHaveLength(2);
    result.forEach(t => expect(t.starred).toBe(true));
  });

  it('starred=false (or absent): all tasks pass starred check', () => {
    const tasks = [
      makeTask({ starred: true }),
      makeTask({ starred: false }),
    ];
    const result = filterTasksForView(tasks, { ...defaultOpts, starred: false });
    expect(result).toHaveLength(2);
  });

  it('starred undefined: all tasks pass starred check', () => {
    const tasks = [makeTask({ starred: true }), makeTask({ starred: false })];
    const result = filterTasksForView(tasks, { ...defaultOpts });
    expect(result).toHaveLength(2);
  });

  it('searchQuery filters by caption (case-insensitive)', () => {
    const tasks = [
      makeTask({ caption: 'Buy groceries' }),
      makeTask({ caption: 'Call dentist' }),
      makeTask({ caption: 'Buy stamps' }),
    ];
    const result = filterTasksForView(tasks, { ...defaultOpts, searchQuery: 'buy' });
    expect(result).toHaveLength(2);
    result.forEach(t => expect(t.caption.toLowerCase()).toContain('buy'));
  });

  it('flagId filter: only matching flag tasks pass', () => {
    const tasks = [
      makeTask({ flag_id: 'f1' }),
      makeTask({ flag_id: 'f2' }),
      makeTask({ flag_id: null }),
    ];
    const result = filterTasksForView(tasks, { ...defaultOpts, flagId: 'f1' });
    expect(result).toHaveLength(1);
    expect(result[0].flag_id).toBe('f1');
  });

  it('flagId __starred__ filters to starred tasks', () => {
    const tasks = [
      makeTask({ starred: true }),
      makeTask({ starred: false }),
    ];
    const result = filterTasksForView(tasks, { ...defaultOpts, flagId: '__starred__' });
    expect(result).toHaveLength(1);
    expect(result[0].starred).toBe(true);
  });

  it('flagId __today__ filters to tasks due today', () => {
    const tasks = [
      makeTask({ due_date: '2026-03-12' }),
      makeTask({ due_date: '2026-03-13' }),
      makeTask({ due_date: null }),
    ];
    const result = filterTasksForView(tasks, { ...defaultOpts, flagId: '__today__' });
    expect(result).toHaveLength(1);
    expect(result[0].due_date).toBe('2026-03-12');
  });
});

// ── actionFilter: 'available' ─────────────────────────────────────────────────

describe("filterTasksForView — actionFilter: 'available'", () => {
  it('excludes completed tasks', () => {
    const tasks = [
      makeTask({ completed_at: null }),
      makeTask({ completed_at: '2026-03-10T00:00:00' }),
    ];
    const result = filterTasksForView(tasks, { ...defaultOpts, actionFilter: 'available' });
    expect(result.every(t => !t.completed_at)).toBe(true);
  });

  it('excludes tasks with has_children=true (non-leaf)', () => {
    const tasks = [
      makeTask({ has_children: false }),
      makeTask({ has_children: true }),
    ];
    const result = filterTasksForView(tasks, { ...defaultOpts, actionFilter: 'available' });
    expect(result).toHaveLength(1);
    expect(result[0].has_children).toBe(false);
  });

  it('excludes folder tasks', () => {
    const tasks = [
      makeTask({ is_folder: false }),
      makeTask({ is_folder: true }),
    ];
    const result = filterTasksForView(tasks, { ...defaultOpts, actionFilter: 'available' });
    expect(result).toHaveLength(1);
    expect(result[0].is_folder).toBe(false);
  });

  it('excludes hidden tasks', () => {
    const tasks = [
      makeTask({ hide_in_views: false }),
      makeTask({ hide_in_views: true }),
    ];
    const result = filterTasksForView(tasks, { ...defaultOpts, actionFilter: 'available' });
    expect(result).toHaveLength(1);
    expect(result[0].hide_in_views).toBe(false);
  });

  it('excludes tasks with future start_date', () => {
    const tasks = [
      makeTask({ start_date: null }),
      makeTask({ start_date: '2026-03-11' }),  // past — OK
      makeTask({ start_date: '2026-03-13' }),  // future — excluded
    ];
    const result = filterTasksForView(tasks, { ...defaultOpts, actionFilter: 'available' });
    expect(result).toHaveLength(2);
  });

  it('excludes tasks whose parent is completed', () => {
    const parent = makeTask({ completed_at: '2026-03-10T00:00:00', has_children: true });
    const child = makeTask({ parent_id: parent.id });
    const result = filterTasksForView([parent, child], { ...defaultOpts, actionFilter: 'available' });
    expect(result.map(t => t.id)).not.toContain(child.id);
  });

  it('excludes tasks whose ancestor is hidden', () => {
    const parent = makeTask({ hide_in_views: true, has_children: true });
    const child = makeTask({ parent_id: parent.id });
    const result = filterTasksForView([parent, child], { ...defaultOpts, actionFilter: 'available' });
    expect(result.map(t => t.id)).not.toContain(child.id);
  });

  it('includes normal leaf task with no constraints', () => {
    const t = makeTask();
    const result = filterTasksForView([t], { ...defaultOpts, actionFilter: 'available' });
    expect(result).toHaveLength(1);
  });
});

// ── actionFilter: 'active' ────────────────────────────────────────────────────

describe("filterTasksForView — actionFilter: 'active'", () => {
  it('active excludes task blocked by subtasks_in_order', () => {
    // parent has subtasks_in_order; sibling before child is not complete
    const parent = makeTask({ subtasks_in_order: true, has_children: true });
    const sibling = makeTask({ parent_id: parent.id, position: 1 });
    const child = makeTask({ parent_id: parent.id, position: 2 });

    const result = filterTasksForView([parent, sibling, child], {
      ...defaultOpts,
      actionFilter: 'active',
    });
    // Only sibling (first) should pass; child is blocked by incomplete sibling
    const ids = result.map(t => t.id);
    expect(ids).toContain(sibling.id);
    expect(ids).not.toContain(child.id);
  });

  it('active includes task when blocking sibling is completed', () => {
    const parent = makeTask({ subtasks_in_order: true, has_children: true });
    const sibling = makeTask({ parent_id: parent.id, position: 1, completed_at: '2026-03-11T00:00:00' });
    const child = makeTask({ parent_id: parent.id, position: 2 });

    const result = filterTasksForView([parent, sibling, child], {
      ...defaultOpts,
      actionFilter: 'active',
    });
    expect(result.map(t => t.id)).toContain(child.id);
  });

  it('active includes task when blocking sibling is a folder', () => {
    const parent = makeTask({ subtasks_in_order: true, has_children: true });
    const sibling = makeTask({ parent_id: parent.id, position: 1, is_folder: true });
    const child = makeTask({ parent_id: parent.id, position: 2 });

    const result = filterTasksForView([parent, sibling, child], {
      ...defaultOpts,
      actionFilter: 'active',
    });
    expect(result.map(t => t.id)).toContain(child.id);
  });

  it('active applies all the same base criteria as available', () => {
    const hidden = makeTask({ hide_in_views: true });
    const folder = makeTask({ is_folder: true });
    const normal = makeTask();

    const result = filterTasksForView([hidden, folder, normal], {
      ...defaultOpts,
      actionFilter: 'active',
    });
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe(normal.id);
  });
});

// ── actionFilter: 'next_actions' ──────────────────────────────────────────────

describe("filterTasksForView — actionFilter: 'next_actions'", () => {
  it('returns only first active task per root project', () => {
    // Two root tasks (no parent), each has one leaf child active
    const root1 = makeTask({ is_project: true, has_children: true });
    const child1a = makeTask({ parent_id: root1.id, position: 1 });
    const child1b = makeTask({ parent_id: root1.id, position: 2 });

    const root2 = makeTask({ is_project: true, has_children: true });
    const child2a = makeTask({ parent_id: root2.id, position: 1 });

    const result = filterTasksForView([root1, child1a, child1b, root2, child2a], {
      ...defaultOpts,
      actionFilter: 'next_actions',
    });
    // Should return child1a (first for root1) and child2a (first for root2)
    const ids = result.map(t => t.id);
    expect(ids).toContain(child1a.id);
    expect(ids).not.toContain(child1b.id);
    expect(ids).toContain(child2a.id);
    expect(result).toHaveLength(2);
  });

  it('single leaf task with no parent is its own project', () => {
    const t = makeTask();
    const result = filterTasksForView([t], { ...defaultOpts, actionFilter: 'next_actions' });
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe(t.id);
  });

  it('completed tasks are excluded from next_actions', () => {
    const t = makeTask({ completed_at: '2026-03-10T00:00:00' });
    const result = filterTasksForView([t], { ...defaultOpts, actionFilter: 'next_actions' });
    expect(result).toHaveLength(0);
  });
});

// ── groupTasks ────────────────────────────────────────────────────────────────

describe('groupTasks', () => {
  it("groupBy='none' returns single group with all tasks", () => {
    const tasks = [makeTask(), makeTask()];
    const groups = groupTasks(tasks, 'none');
    expect(groups).toHaveLength(1);
    expect(groups[0].key).toBe('__all__');
    expect(groups[0].tasks).toHaveLength(2);
  });

  it("groupBy='' returns single group with all tasks", () => {
    const tasks = [makeTask(), makeTask()];
    const groups = groupTasks(tasks, '');
    expect(groups).toHaveLength(1);
  });

  it("groupBy='flag' groups by flag correctly", () => {
    const tasks = [
      makeTask({ flag: { id: 'f1', name: 'Work', color: '#f00', position: 0 }, flag_id: 'f1' }),
      makeTask({ flag: null, flag_id: null }),
      makeTask({ flag: { id: 'f1', name: 'Work', color: '#f00', position: 0 }, flag_id: 'f1' }),
    ];
    const groups = groupTasks(tasks, 'flag');
    const workGroup = groups.find(g => g.key === 'f1');
    const noFlagGroup = groups.find(g => g.key === '__noflag__');
    expect(workGroup?.tasks).toHaveLength(2);
    expect(noFlagGroup?.tasks).toHaveLength(1);
  });

  it("groupBy='due_date' groups overdue/today/week/later/nodue in order", () => {
    const tasks = [
      makeTask({ due_date: '2026-03-01' }),   // overdue
      makeTask({ due_date: '2026-03-12' }),   // today
      makeTask({ due_date: '2026-03-15' }),   // this week
      makeTask({ due_date: '2026-04-30' }),   // later
      makeTask({ due_date: null }),           // no due
    ];
    const groups = groupTasks(tasks, 'due_date');
    const keys = groups.map(g => g.key);
    expect(keys).toEqual(['__overdue__', '__today__', '__week__', '__later__', '__nodue__']);
  });

  it("groupBy='tag' allows task to appear in multiple tag groups", () => {
    const tag1 = { id: 't1', name: 'urgent', color: '#f00' };
    const tag2 = { id: 't2', name: 'home', color: '#00f' };
    const tasks = [
      makeTask({ tags: [tag1, tag2] }),
      makeTask({ tags: [tag1] }),
      makeTask({ tags: [] }),
    ];
    const groups = groupTasks(tasks, 'tag');
    const urgentGroup = groups.find(g => g.key === 't1');
    const homeGroup = groups.find(g => g.key === 't2');
    const noTagGroup = groups.find(g => g.key === '__notag__');
    expect(urgentGroup?.tasks).toHaveLength(2);
    expect(homeGroup?.tasks).toHaveLength(1);
    expect(noTagGroup?.tasks).toHaveLength(1);
  });

  it("groupBy='start_date' groups by start_date value or nostart", () => {
    const tasks = [
      makeTask({ start_date: '2026-03-10' }),
      makeTask({ start_date: '2026-03-10' }),
      makeTask({ start_date: null }),
    ];
    const groups = groupTasks(tasks, 'start_date');
    const dateGroup = groups.find(g => g.key === '2026-03-10');
    const noStartGroup = groups.find(g => g.key === '__nostart__');
    expect(dateGroup?.tasks).toHaveLength(2);
    expect(noStartGroup?.tasks).toHaveLength(1);
  });
});
