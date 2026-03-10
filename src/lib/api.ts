import { invoke } from '@tauri-apps/api/core';
import type { Task, Flag, Tag, SavedView } from './types';

export const api = {
  // Tasks
  getTasks: (parent_id: string | null = null): Promise<Task[]> =>
    invoke('get_tasks', { parentId: parent_id }),
  getAllFlat: (include_completed = false): Promise<Task[]> =>
    invoke('get_all_tasks_flat', { includeCompleted: include_completed }),
  createTask: (input: object): Promise<Task> => invoke('create_task', { input }),
  updateTask: (id: string, input: object): Promise<Task> => invoke('update_task', { id, input }),
  deleteTask: (id: string): Promise<void> => invoke('delete_task', { id }),
  completeTask: (id: string, completed: boolean): Promise<Task> => invoke('complete_task', { id, completed }),
  moveTask: (id: string, newParentId: string | null, newPosition: number): Promise<Task> =>
    invoke('move_task', { id, newParentId, newPosition }),
  reorderTasks: (idsAndPositions: [string, number][]): Promise<void> =>
    invoke('reorder_tasks', { idsAndPositions }),

  // Flags
  getFlags: (): Promise<Flag[]> => invoke('get_flags'),
  createFlag: (name: string, color: string): Promise<Flag> => invoke('create_flag', { name, color }),
  updateFlag: (id: string, name: string, color: string): Promise<Flag> => invoke('update_flag', { id, name, color }),
  deleteFlag: (id: string): Promise<void> => invoke('delete_flag', { id }),

  // Tags
  getTags: (): Promise<Tag[]> => invoke('get_tags'),
  createTag: (name: string, color: string): Promise<Tag> => invoke('create_tag', { name, color }),
  deleteTag: (id: string): Promise<void> => invoke('delete_tag', { id }),

  // Email links
  addEmailLink: (taskId: string, linkType: string, linkData: string, subject?: string): Promise<string> =>
    invoke('add_email_link', { taskId, linkType, linkData, subject }),
  deleteEmailLink: (id: string): Promise<void> => invoke('delete_email_link', { id }),

  // Views
  getViews: (): Promise<SavedView[]> => invoke('get_views'),
  createView: (payload: object): Promise<SavedView> => invoke('create_view', { payload }),
  updateView: (id: string, payload: object): Promise<SavedView> => invoke('update_view', { id, payload }),
  deleteView: (id: string): Promise<void> => invoke('delete_view', { id }),

  // GDrive
  gdriveAuthStatus: (): Promise<boolean> => invoke('gdrive_auth_status'),
  gdriveConnect: (): Promise<{ success: boolean; message: string; synced_at?: string }> => invoke('gdrive_connect'),
  gdriveUpload: (): Promise<{ success: boolean; message: string; synced_at?: string }> => invoke('gdrive_upload'),
  gdriveDownload: (): Promise<{ success: boolean; message: string; synced_at?: string }> => invoke('gdrive_download'),
};
