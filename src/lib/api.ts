import { invoke } from '@tauri-apps/api/core';
import type {
  Task, CreateTaskInput, UpdateTaskInput,
  Flag, Tag, SavedView, ViewPayload,
} from './types';

export const api = {
  // ── Tasks ──────────────────────────────────────────────────────────────
  getTasks: (parent_id: string | null = null): Promise<Task[]> =>
    invoke('get_tasks', { parentId: parent_id }),

  getAllFlat: (include_completed = false): Promise<Task[]> =>
    invoke('get_all_tasks_flat', { includeCompleted: include_completed }),

  createTask: (input: CreateTaskInput): Promise<Task> =>
    invoke('create_task', { input }),

  updateTask: (id: string, input: UpdateTaskInput): Promise<Task> =>
    invoke('update_task', { id, input }),

  deleteTask: (id: string): Promise<void> =>
    invoke('delete_task', { id }),

  deleteTaskRecursive: (id: string): Promise<void> =>
    invoke('delete_task_recursive', { id }),

  completeTask: (id: string, completed: boolean): Promise<Task> =>
    invoke('complete_task', { id, completed }),

  completeBranch: (id: string, completed: boolean): Promise<void> =>
    invoke('complete_branch', { id, completed }),

  moveTask: (id: string, newParentId: string | null, newPosition: number): Promise<Task> =>
    invoke('move_task', { id, newParentId, newPosition }),

  reorderTasks: (idsAndPositions: [string, number][]): Promise<void> =>
    invoke('reorder_tasks', { idsAndPositions }),

  duplicateTask: (id: string): Promise<Task> =>
    invoke('duplicate_task', { id }),

  sortSubtasks: (parentId: string | null, sortBy: string, sortDir: string): Promise<void> =>
    invoke('sort_subtasks', { parentId, sortBy, sortDir }),

  // ── Flags ──────────────────────────────────────────────────────────────
  getFlags: (): Promise<Flag[]> =>
    invoke('get_flags'),

  createFlag: (name: string, color: string): Promise<Flag> =>
    invoke('create_flag', { name, color }),

  updateFlag: (id: string, name: string, color: string): Promise<Flag> =>
    invoke('update_flag', { id, name, color }),

  deleteFlag: (id: string): Promise<void> =>
    invoke('delete_flag', { id }),

  reorderFlags: (idsAndPositions: [string, number][]): Promise<void> =>
    invoke('reorder_flags', { idsAndPositions }),

  // ── Tags ───────────────────────────────────────────────────────────────
  getTags: (): Promise<Tag[]> =>
    invoke('get_tags'),

  createTag: (name: string, color: string): Promise<Tag> =>
    invoke('create_tag', { name, color }),

  updateTag: (id: string, name: string, color: string): Promise<Tag> =>
    invoke('update_tag', { id, name, color }),

  deleteTag: (id: string): Promise<void> =>
    invoke('delete_tag', { id }),

  // ── Views ──────────────────────────────────────────────────────────────
  getViews: (): Promise<SavedView[]> =>
    invoke('get_views'),

  createView: (payload: ViewPayload): Promise<SavedView> =>
    invoke('create_view', { payload }),

  updateView: (id: string, payload: ViewPayload): Promise<SavedView> =>
    invoke('update_view', { id, payload }),

  deleteView: (id: string): Promise<void> =>
    invoke('delete_view', { id }),

  reorderViews: (idsAndPositions: [string, number][]): Promise<void> =>
    invoke('reorder_views', { idsAndPositions }),

  // ── Email links ────────────────────────────────────────────────────────
  addEmailLink: (taskId: string, linkType: string, linkData: string, subject?: string): Promise<string> =>
    invoke('add_email_link', { taskId, linkType, linkData, subject }),

  deleteEmailLink: (id: string): Promise<void> =>
    invoke('delete_email_link', { id }),

  // ── Settings ───────────────────────────────────────────────────────────
  getSetting: (key: string): Promise<string | null> =>
    invoke('get_setting', { key }),

  setSetting: (key: string, value: string): Promise<void> =>
    invoke('set_setting', { key, value }),

  getAllSettings: (): Promise<Record<string, string>> =>
    invoke('get_all_settings'),
};
