//! Backend abstraction layer for multiple task management systems
//!
//! This module provides a unified interface for different task management backends
//! (Todoist, local storage, CalDAV, etc.) allowing the application to work with
//! multiple data sources transparently.
//!
//! The abstraction consists of two main traits:
//! - `Backend`: Core CRUD operations for tasks, projects, labels, and sections
//! - `BackendProvider`: Backend metadata and lifecycle management

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Todoist backend implementation
pub mod todoist;

// Re-export common types that backends will use
pub use crate::todoist::{LabelDisplay, ProjectDisplay, SectionDisplay, TaskDisplay};

/// Generic argument structures for backend operations
/// These provide a backend-agnostic way to pass data for create/update operations

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectArgs {
    pub name: String,
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProjectArgs {
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskArgs {
    pub content: String,
    pub description: Option<String>,
    pub project_id: Option<String>,
    pub section_id: Option<String>,
    pub parent_id: Option<String>,
    pub priority: Option<i32>,
    pub labels: Option<Vec<String>>,
    pub due_string: Option<String>,
    pub due_date: Option<String>,
    pub due_datetime: Option<String>,
    pub deadline_date: Option<String>,
    pub duration: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTaskArgs {
    pub content: Option<String>,
    pub description: Option<String>,
    pub labels: Option<Vec<String>>,
    pub priority: Option<i32>,
    pub due_string: Option<String>,
    pub due_date: Option<String>,
    pub due_datetime: Option<String>,
    pub deadline_date: Option<String>,
    pub duration: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLabelArgs {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateLabelArgs {
    pub name: Option<String>,
}

/// Status information for a backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendStatus {
    pub backend_id: String,
    pub backend_name: String,
    pub backend_type: String,
    pub is_authenticated: bool,
    pub is_connected: bool,
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
    pub sync_error: Option<String>,
}

/// Core backend trait defining CRUD operations for task management
///
/// This trait provides a unified interface for all backend implementations,
/// allowing the sync service to work with different backends transparently.
/// All methods return the standard display models used throughout the application.
#[async_trait]
pub trait Backend: Send + Sync {
    /// Retrieve all projects from the backend
    async fn get_projects(&self) -> Result<Vec<ProjectDisplay>>;

    /// Retrieve all tasks from the backend
    async fn get_tasks(&self) -> Result<Vec<TaskDisplay>>;

    /// Retrieve all labels from the backend
    async fn get_labels(&self) -> Result<Vec<LabelDisplay>>;

    /// Retrieve all sections from the backend
    async fn get_sections(&self) -> Result<Vec<SectionDisplay>>;

    /// Create a new project in the backend
    async fn create_project(&self, args: &CreateProjectArgs) -> Result<ProjectDisplay>;

    /// Create a new task in the backend
    async fn create_task(&self, args: &CreateTaskArgs) -> Result<TaskDisplay>;

    /// Create a new label in the backend
    async fn create_label(&self, args: &CreateLabelArgs) -> Result<LabelDisplay>;

    /// Update an existing project in the backend
    async fn update_project(&self, id: &str, args: &UpdateProjectArgs) -> Result<ProjectDisplay>;

    /// Update an existing task in the backend
    async fn update_task(&self, id: &str, args: &UpdateTaskArgs) -> Result<TaskDisplay>;

    /// Update an existing label in the backend
    async fn update_label(&self, id: &str, args: &UpdateLabelArgs) -> Result<LabelDisplay>;

    /// Delete a project from the backend
    async fn delete_project(&self, id: &str) -> Result<()>;

    /// Delete a task from the backend
    async fn delete_task(&self, id: &str) -> Result<()>;

    /// Delete a label from the backend
    async fn delete_label(&self, id: &str) -> Result<()>;

    /// Mark a task as completed in the backend
    async fn complete_task(&self, id: &str) -> Result<()>;

    /// Reopen a completed task in the backend
    async fn reopen_task(&self, id: &str) -> Result<()>;
}

/// Backend provider trait for metadata and lifecycle management
///
/// This trait provides backend identification, authentication status,
/// and connection testing capabilities. It's used by the sync service
/// to manage multiple backends and their states.
#[async_trait]
pub trait BackendProvider: Send + Sync {
    /// Unique identifier for this backend instance
    fn backend_id(&self) -> &str;

    /// Human-readable name for this backend
    fn backend_name(&self) -> &str;

    /// Backend type identifier (e.g., "todoist", "caldav", "local")
    fn backend_type(&self) -> &str;

    /// Check if the backend is properly authenticated
    fn is_authenticated(&self) -> bool;

    /// Test connectivity to the backend service
    /// Returns Ok(()) if connection is successful, Err otherwise
    async fn test_connection(&self) -> Result<()>;

    /// Get current status of the backend
    async fn get_status(&self) -> Result<BackendStatus>;
}

/// Combined trait for a complete backend implementation
///
/// Most backend implementations will implement both Backend and BackendProvider,
/// so this trait provides a convenient way to require both at once.
pub trait FullBackend: Backend + BackendProvider {}

// Automatic implementation for any type that implements both traits
impl<T> FullBackend for T where T: Backend + BackendProvider {}

/// Error types specific to backend operations
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("Backend '{0}' not found")]
    BackendNotFound(String),

    #[error("Backend '{0}' is not authenticated")]
    NotAuthenticated(String),

    #[error("Backend '{0}' connection failed: {1}")]
    ConnectionFailed(String, String),

    #[error("Backend '{0}' operation failed: {1}")]
    OperationFailed(String, String),

    #[error("Invalid backend configuration: {0}")]
    InvalidConfiguration(String),
}
