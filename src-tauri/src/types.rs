use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flag {
    pub id: String,
    pub name: String,
    pub color: String,
    pub position: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailLink {
    pub id: String,
    pub task_id: String,
    pub link_type: String,
    pub link_data: String,
    pub subject: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub parent_id: Option<String>,
    pub caption: String,
    pub note: String,
    pub position: f64,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub start_date: Option<String>,
    pub due_date: Option<String>,
    pub reminder_at: Option<String>,
    pub recurrence_rule: Option<String>,
    pub flag_id: Option<String>,
    pub flag: Option<Flag>,
    pub starred: bool,
    pub color: Option<String>,
    pub is_folder: bool,
    pub is_project: bool,
    pub hide_in_views: bool,
    pub subtasks_in_order: bool,
    pub inherit_dates: bool,
    pub custom_format: Option<String>,
    pub tags: Vec<Tag>,
    pub email_links: Vec<EmailLink>,
    pub has_children: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedView {
    pub id: String,
    pub name: String,
    pub show_completed: bool,
    pub group_by: String,
    pub sort_by: String,
    pub sort_dir: String,
    pub visible_fields: Vec<String>,
    pub filter_json: String,
    pub position: f64,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskInput {
    pub parent_id: Option<String>,
    pub caption: String,
    pub note: Option<String>,
    pub position: Option<f64>,
    pub start_date: Option<String>,
    pub due_date: Option<String>,
    pub reminder_at: Option<String>,
    pub flag_id: Option<String>,
    pub starred: Option<bool>,
    pub tag_ids: Option<Vec<String>>,
    pub is_folder: Option<bool>,
    pub is_project: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskInput {
    pub caption: Option<String>,
    pub note: Option<String>,
    pub start_date: Option<String>,
    pub due_date: Option<String>,
    pub reminder_at: Option<String>,
    pub recurrence_rule: Option<String>,
    pub flag_id: Option<String>,
    pub starred: Option<bool>,
    pub color: Option<String>,
    pub is_folder: Option<bool>,
    pub is_project: Option<bool>,
    pub hide_in_views: Option<bool>,
    pub subtasks_in_order: Option<bool>,
    pub inherit_dates: Option<bool>,
    pub custom_format: Option<String>,
    pub tag_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct ViewPayload {
    pub name: String,
    pub show_completed: bool,
    pub group_by: String,
    pub sort_by: String,
    pub sort_dir: String,
    pub visible_fields: Vec<String>,
    pub filter_json: String,
}

#[derive(Debug, Serialize)]
pub struct SyncResult {
    pub success: bool,
    pub message: String,
    pub synced_at: Option<String>,
}

pub type AppSettings = HashMap<String, String>;
