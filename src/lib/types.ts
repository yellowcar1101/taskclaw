export interface Flag {
  id: string;
  name: string;
  color: string;
  position: number;
}

export interface Tag {
  id: string;
  name: string;
  color: string;
}

export interface EmailLink {
  id: string;
  task_id: string;
  link_type: 'message_id' | 'thread_id' | 'mailto';
  link_data: string;
  subject?: string;
}

export interface Task {
  id: string;
  parent_id: string | null;
  caption: string;
  note: string;
  position: number;
  created_at: string;
  updated_at: string;
  completed_at: string | null;
  start_date: string | null;
  due_date: string | null;
  reminder_at: string | null;
  recurrence_rule: string | null;
  flag_id: string | null;
  flag: Flag | null;
  starred: boolean;
  color: string | null;
  is_folder: boolean;
  is_project: boolean;
  hide_in_views: boolean;
  subtasks_in_order: boolean;
  inherit_dates: boolean;
  custom_format: string | null;
  tags: Tag[];
  email_links: EmailLink[];
  has_children: boolean;
}

export interface CreateTaskInput {
  parent_id?: string | null;
  caption: string;
  note?: string;
  position?: number;
  start_date?: string;
  due_date?: string;
  reminder_at?: string;
  flag_id?: string | null;
  starred?: boolean;
  tag_ids?: string[];
  is_folder?: boolean;
  is_project?: boolean;
}

export interface UpdateTaskInput {
  caption?: string;
  note?: string;
  start_date?: string;
  due_date?: string;
  reminder_at?: string;
  recurrence_rule?: string;
  flag_id?: string;
  starred?: boolean;
  color?: string;
  is_folder?: boolean;
  is_project?: boolean;
  hide_in_views?: boolean;
  subtasks_in_order?: boolean;
  inherit_dates?: boolean;
  custom_format?: string;
  tag_ids?: string[];
}

export interface ViewPayload {
  name: string;
  show_completed: boolean;
  group_by: string;
  sort_by: string;
  sort_dir: string;
  visible_fields: string[];
  filter_json: string;
}

export interface SavedView {
  id: string;
  name: string;
  show_completed: boolean;
  group_by: string;
  sort_by: string;
  sort_dir: string;
  visible_fields: string[];
  filter_json: string;
  position: number;
}

export type SortField = 'position' | 'caption' | 'due_date' | 'start_date' | 'starred';
export type SortDir = 'asc' | 'desc';
