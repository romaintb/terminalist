use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const TODOIST_API_BASE: &str = "https://api.todoist.com/rest/v2";

/// A simplified wrapper around the Todoist REST API v2
#[derive(Clone)]
pub struct TodoistWrapper {
    client: Client,
    api_token: String,
}

impl TodoistWrapper {
    /// Create a new Todoist wrapper with your API token
    /// You can get your API token from: https://todoist.com/prefs/integrations
    pub fn new(api_token: String) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { client, api_token }
    }

    /// Get all projects
    pub async fn get_projects(&self) -> Result<Vec<Project>> {
        let url = format!("{TODOIST_API_BASE}/projects");
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await?;

        let projects: Vec<Project> = response.json().await?;
        Ok(projects)
    }

    /// Get all tasks
    pub async fn get_tasks(&self) -> Result<Vec<Task>> {
        let url = format!("{TODOIST_API_BASE}/tasks");
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await?;

        let tasks: Vec<Task> = response.json().await?;
        Ok(tasks)
    }

    /// Get tasks for a specific project
    pub async fn get_tasks_for_project(&self, project_id: &str) -> Result<Vec<Task>> {
        let url = format!("{TODOIST_API_BASE}/tasks?project_id={project_id}");
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await?;

        let tasks: Vec<Task> = response.json().await?;
        Ok(tasks)
    }

    /// Create a new task
    pub async fn create_task(&self, content: &str, project_id: Option<&str>) -> Result<Task> {
        let url = format!("{TODOIST_API_BASE}/tasks");

        let mut body = HashMap::new();
        body.insert("content", content);
        if let Some(pid) = project_id {
            body.insert("project_id", pid);
        }

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let task: Task = response.json().await?;
        Ok(task)
    }

    /// Complete a task
    pub async fn complete_task(&self, task_id: &str) -> Result<()> {
        let url = format!("{TODOIST_API_BASE}/tasks/{task_id}/close");
        self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await?;

        Ok(())
    }

    /// Reopen a completed task
    pub async fn reopen_task(&self, task_id: &str) -> Result<()> {
        let url = format!("{TODOIST_API_BASE}/tasks/{task_id}/reopen");
        self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await?;

        Ok(())
    }

    /// Delete a task
    pub async fn delete_task(&self, task_id: &str) -> Result<()> {
        let url = format!("{TODOIST_API_BASE}/tasks/{task_id}");
        self.client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await?;

        Ok(())
    }

    /// Get all labels
    pub async fn get_labels(&self) -> Result<Vec<Label>> {
        let url = format!("{TODOIST_API_BASE}/labels");
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await?;

        let labels: Vec<Label> = response.json().await?;
        Ok(labels)
    }

    /// Update task content
    pub async fn update_task(&self, task_id: &str, content: &str) -> Result<Task> {
        let url = format!("{TODOIST_API_BASE}/tasks/{task_id}");

        let mut body = HashMap::new();
        body.insert("content", content);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let task: Task = response.json().await?;
        Ok(task)
    }
}

/// Todoist Task model
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: String,
    pub content: String,
    pub description: String,
    pub project_id: String,
    pub section_id: Option<String>,
    pub parent_id: Option<String>,
    pub order: i32,
    pub priority: i32,
    pub is_completed: bool,
    pub labels: Vec<String>,
    pub created_at: String,
    pub due: Option<Due>,
    pub deadline: Option<Deadline>,
    pub duration: Option<Duration>,
    pub assignee_id: Option<String>,
    pub url: String,
    pub comment_count: i32,
}

/// Todoist Project model
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub comment_count: i32,
    pub order: i32,
    pub color: String,
    pub is_shared: bool,
    pub is_favorite: bool,
    pub is_inbox_project: bool,
    pub is_team_inbox: bool,
    pub view_style: String,
    pub url: String,
    pub parent_id: Option<String>,
}

/// Todoist Label model
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Label {
    pub id: String,
    pub name: String,
    pub color: String,
    pub order: i32,
    pub is_favorite: bool,
}

/// Todoist Due date model
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Due {
    pub string: String,
    pub date: String,
    pub is_recurring: bool,
    pub datetime: Option<String>,
    pub timezone: Option<String>,
}

/// Todoist Deadline model
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Deadline {
    pub date: String,
}

/// Todoist Duration model
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Duration {
    pub amount: i32,
    pub unit: String, // "minute", "hour", "day"
}

/// Helper struct for displaying tasks in a user-friendly way
#[derive(Debug, Serialize, Deserialize)]
pub struct TaskDisplay {
    pub id: String,
    pub content: String,
    pub project_id: String,
    pub is_completed: bool,
    pub is_deleted: bool,
    pub priority: i32,
    pub due: Option<String>,
    pub due_datetime: Option<String>,
    pub is_recurring: bool,
    pub deadline: Option<String>,
    pub duration: Option<String>,
    pub labels: Vec<String>,
    pub description: String,
}

impl From<Task> for TaskDisplay {
    fn from(task: Task) -> Self {
        let duration_string = task.duration.map(|d| match d.unit.as_str() {
            "minute" => format!("{}m", d.amount),
            "hour" => format!("{}h", d.amount),
            "day" => format!("{}d", d.amount),
            _ => format!("{} {}", d.amount, d.unit),
        });

        Self {
            id: task.id,
            content: task.content,
            project_id: task.project_id,
            is_completed: task.is_completed,
            is_deleted: false, // Tasks from API are not deleted
            priority: task.priority,
            due: task.due.as_ref().map(|d| d.string.clone()),
            due_datetime: task.due.as_ref().and_then(|d| d.datetime.clone()),
            is_recurring: task.due.as_ref().map(|d| d.is_recurring).unwrap_or(false),
            deadline: task.deadline.map(|d| d.date),
            duration: duration_string,
            labels: task.labels,
            description: task.description,
        }
    }
}

/// Helper struct for displaying projects in a user-friendly way
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectDisplay {
    pub id: String,
    pub name: String,
    pub color: String,
    pub is_favorite: bool,
}

impl From<Project> for ProjectDisplay {
    fn from(project: Project) -> Self {
        Self {
            id: project.id,
            name: project.name,
            color: project.color,
            is_favorite: project.is_favorite,
        }
    }
}
