import { invoke } from '@tauri-apps/api/core';
import type { Task, CreateTaskInput, UpdateTaskInput, Context } from './types';

export const api = {
  getTasks: (parent_id: string | null = null): Promise<Task[]> =>
    invoke('get_tasks', { parentId: parent_id }),

  getAllFlat: (): Promise<Task[]> =>
    invoke('get_all_tasks_flat'),

  createTask: (input: CreateTaskInput): Promise<Task> =>
    invoke('create_task', { input }),

  updateTask: (id: string, input: UpdateTaskInput): Promise<Task> =>
    invoke('update_task', { id, input }),

  deleteTask: (id: string): Promise<void> =>
    invoke('delete_task', { id }),

  completeTask: (id: string, completed: boolean): Promise<Task> =>
    invoke('complete_task', { id, completed }),

  moveTask: (id: string, newParentId: string | null, newPosition: number): Promise<Task> =>
    invoke('move_task', { id, newParentId, newPosition }),

  reorderTasks: (idsAndPositions: [string, number][]): Promise<void> =>
    invoke('reorder_tasks', { idsAndPositions }),

  getContexts: (): Promise<Context[]> =>
    invoke('get_contexts'),

  createContext: (name: string, color: string): Promise<Context> =>
    invoke('create_context', { name, color }),

  deleteContext: (id: string): Promise<void> =>
    invoke('delete_context', { id }),

  addEmailLink: (taskId: string, linkType: string, linkData: string, subject?: string): Promise<string> =>
    invoke('add_email_link', { taskId, linkType, linkData, subject }),
};
