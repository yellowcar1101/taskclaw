export interface Context {
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
  importance: number;
  urgency: number;
  effort: number;
  due_date: string | null;
  reminder_at: string | null;
  recurrence_rule: string | null;
  starred: boolean;
  color: string | null;
  contexts: Context[];
  tags: Tag[];
  email_links: EmailLink[];
  has_children: boolean;
  score: number;
}

export interface CreateTaskInput {
  parent_id?: string | null;
  caption: string;
  note?: string;
  position?: number;
  importance?: number;
  urgency?: number;
  effort?: number;
  due_date?: string;
  starred?: boolean;
  context_ids?: string[];
}

export interface UpdateTaskInput {
  caption?: string;
  note?: string;
  importance?: number;
  urgency?: number;
  effort?: number;
  due_date?: string;
  reminder_at?: string;
  recurrence_rule?: string;
  starred?: boolean;
  color?: string;
  context_ids?: string[];
  tag_ids?: string[];
}

export type SortField = 'position' | 'caption' | 'due_date' | 'score' | 'importance' | 'urgency';
export type SortDir = 'asc' | 'desc';
