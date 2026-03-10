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
  link_type: 'gmail' | 'outlook';
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
  tags: Tag[];
  email_links: EmailLink[];
  has_children: boolean;
}

export interface SavedView {
  id: string;
  name: string;
  show_completed: boolean;
  group_by: GroupByField;
  sort_by: SortByField;
  sort_dir: 'asc' | 'desc';
  visible_fields: string[];
  position: number;
}

export type GroupByField = 'none' | 'flag' | 'tag' | 'due_date' | 'start_date' | 'created_at' | 'updated_at';
export type SortByField = 'position' | 'caption' | 'start_date' | 'due_date' | 'created_at' | 'updated_at' | 'flag';

export interface TaskGroup {
  key: string;
  label: string;
  color?: string;
  tasks: Task[];
  collapsed: boolean;
}
