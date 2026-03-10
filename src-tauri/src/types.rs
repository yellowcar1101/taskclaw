use serde::{Deserialize, Serialize};

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
    pub importance: i32,
    pub urgency: i32,
    pub effort: i32,
    pub due_date: Option<String>,
    pub reminder_at: Option<String>,
    pub recurrence_rule: Option<String>,
    pub starred: bool,
    pub color: Option<String>,
    pub contexts: Vec<Context>,
    pub tags: Vec<Tag>,
    pub email_links: Vec<EmailLink>,
    pub has_children: bool,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
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
pub struct SavedView {
    pub id: String,
    pub name: String,
    pub filter_json: String,
    pub sort_field: String,
    pub sort_dir: String,
    pub position: f64,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskInput {
    pub parent_id: Option<String>,
    pub caption: String,
    pub note: Option<String>,
    pub position: Option<f64>,
    pub importance: Option<i32>,
    pub urgency: Option<i32>,
    pub effort: Option<i32>,
    pub due_date: Option<String>,
    pub starred: Option<bool>,
    pub color: Option<String>,
    pub context_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskInput {
    pub caption: Option<String>,
    pub note: Option<String>,
    pub importance: Option<i32>,
    pub urgency: Option<i32>,
    pub effort: Option<i32>,
    pub due_date: Option<String>,
    pub reminder_at: Option<String>,
    pub recurrence_rule: Option<String>,
    pub starred: Option<bool>,
    pub color: Option<String>,
    pub context_ids: Option<Vec<String>>,
    pub tag_ids: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct SyncResult {
    pub success: bool,
    pub message: String,
    pub synced_at: Option<String>,
}

pub fn compute_score(importance: i32, urgency: i32, due_date: &Option<String>) -> f64 {
    let base = (importance as f64 * 0.6 + urgency as f64 * 0.4) * 20.0;
    let due_boost = if let Some(due) = due_date {
        let now = chrono::Utc::now().date_naive();
        if let Ok(d) = chrono::NaiveDate::parse_from_str(due, "%Y-%m-%d") {
            let days = (d - now).num_days();
            if days < 0 {
                20.0
            } else if days == 0 {
                15.0
            } else if days <= 3 {
                10.0
            } else if days <= 7 {
                5.0
            } else {
                0.0
            }
        } else {
            0.0
        }
    } else {
        0.0
    };
    (base + due_boost).min(100.0)
}
