//! Backend abstraction layer for multi-backend support.
//!
//! This module defines the common interface that all task management backends must implement,
//! along with common data types and error handling.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub mod todoist;

/// Common error types for backend operations.
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Backend error: {0}")]
    Other(String),
}

/// Backend-agnostic project representation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackendProject {
    pub remote_id: String,
    pub name: String,
    pub color: String,
    pub is_favorite: bool,
    pub is_inbox: bool,
    pub order_index: i32,
    pub parent_remote_id: Option<String>,
}

/// Backend-agnostic task representation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackendTask {
    pub remote_id: String,
    pub content: String,
    pub description: Option<String>,
    pub project_remote_id: String,
    pub section_remote_id: Option<String>,
    pub parent_remote_id: Option<String>,
    pub priority: i32,
    pub order_index: i32,
    pub due_date: Option<String>,
    pub due_datetime: Option<String>,
    pub is_recurring: bool,
    pub deadline: Option<String>,
    pub duration: Option<String>,
    pub is_completed: bool,
    pub labels: Vec<String>,
}

/// Backend-agnostic label representation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackendLabel {
    pub remote_id: String,
    pub name: String,
    pub color: String,
    pub order_index: i32,
    pub is_favorite: bool,
}

/// Backend-agnostic section representation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackendSection {
    pub remote_id: String,
    pub name: String,
    pub project_remote_id: String,
    pub order_index: i32,
}

/// Arguments for creating a new project.
#[derive(Clone, Debug)]
pub struct CreateProjectArgs {
    pub name: String,
    pub color: Option<String>,
    pub is_favorite: Option<bool>,
    pub parent_remote_id: Option<String>,
}

/// Arguments for creating a new task.
#[derive(Clone, Debug)]
pub struct CreateTaskArgs {
    pub content: String,
    pub description: Option<String>,
    pub project_remote_id: String,
    pub section_remote_id: Option<String>,
    pub parent_remote_id: Option<String>,
    pub priority: Option<i32>,
    pub due_date: Option<String>,
    pub due_datetime: Option<String>,
    pub duration: Option<String>,
    pub labels: Vec<String>,
}

/// Arguments for creating a new label.
#[derive(Clone, Debug)]
pub struct CreateLabelArgs {
    pub name: String,
    pub color: Option<String>,
    pub is_favorite: Option<bool>,
}

/// Arguments for updating a project.
#[derive(Clone, Debug)]
pub struct UpdateProjectArgs {
    pub name: Option<String>,
    pub color: Option<String>,
    pub is_favorite: Option<bool>,
}

/// Arguments for updating a task.
#[derive(Clone, Debug)]
pub struct UpdateTaskArgs {
    pub content: Option<String>,
    pub description: Option<String>,
    pub project_remote_id: Option<String>,
    pub section_remote_id: Option<String>,
    pub parent_remote_id: Option<String>,
    pub priority: Option<i32>,
    pub due_date: Option<String>,
    pub due_datetime: Option<String>,
    pub duration: Option<String>,
    pub labels: Option<Vec<String>>,
}

/// Arguments for updating a label.
#[derive(Clone, Debug)]
pub struct UpdateLabelArgs {
    pub name: Option<String>,
    pub color: Option<String>,
    pub is_favorite: Option<bool>,
}

/// Backend trait that all task management backends must implement.
///
/// This trait defines the common interface for interacting with different
/// task management services (Todoist, TickTick, GitHub, etc.).
#[async_trait]
pub trait Backend: Send + Sync {
    /// Returns the backend type identifier (e.g., "todoist", "ticktick").
    fn backend_type(&self) -> &str;

    // Sync operations - fetch all data
    async fn fetch_projects(&self) -> Result<Vec<BackendProject>, BackendError>;
    async fn fetch_tasks(&self) -> Result<Vec<BackendTask>, BackendError>;
    async fn fetch_labels(&self) -> Result<Vec<BackendLabel>, BackendError>;
    async fn fetch_sections(&self) -> Result<Vec<BackendSection>, BackendError>;

    // CRUD operations for projects
    async fn create_project(&self, args: CreateProjectArgs) -> Result<BackendProject, BackendError>;
    async fn update_project(&self, remote_id: &str, args: UpdateProjectArgs) -> Result<BackendProject, BackendError>;
    async fn delete_project(&self, remote_id: &str) -> Result<(), BackendError>;

    // CRUD operations for tasks
    async fn create_task(&self, args: CreateTaskArgs) -> Result<BackendTask, BackendError>;
    async fn update_task(&self, remote_id: &str, args: UpdateTaskArgs) -> Result<BackendTask, BackendError>;
    async fn delete_task(&self, remote_id: &str) -> Result<(), BackendError>;
    async fn complete_task(&self, remote_id: &str) -> Result<(), BackendError>;
    async fn reopen_task(&self, remote_id: &str) -> Result<(), BackendError>;

    // CRUD operations for labels
    async fn create_label(&self, args: CreateLabelArgs) -> Result<BackendLabel, BackendError>;
    async fn update_label(&self, remote_id: &str, args: UpdateLabelArgs) -> Result<BackendLabel, BackendError>;
    async fn delete_label(&self, remote_id: &str) -> Result<(), BackendError>;
}
