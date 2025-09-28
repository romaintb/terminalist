//! Todoist backend implementation
//!
//! This module implements the Backend and BackendProvider traits for Todoist,
//! wrapping the existing TodoistWrapper to provide a unified backend interface.

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;

use super::{
    Backend, BackendProvider, BackendStatus, CreateLabelArgs, CreateProjectArgs, CreateTaskArgs,
    UpdateLabelArgs, UpdateProjectArgs, UpdateTaskArgs,
};
use crate::todoist::{CreateProjectArgs as TodoistCreateProjectArgs, TodoistWrapper};

/// Todoist backend implementation
///
/// This struct wraps the existing TodoistWrapper to implement the unified backend traits.
/// It provides a bridge between the new backend abstraction and the existing Todoist integration.
pub struct TodoistBackend {
    /// The underlying Todoist API wrapper
    wrapper: TodoistWrapper,
    /// Unique identifier for this backend instance
    backend_id: String,
    /// Human-readable name for this backend
    backend_name: String,
}

impl TodoistBackend {
    /// Create a new TodoistBackend instance
    ///
    /// # Arguments
    /// * `api_token` - The Todoist API token for authentication
    /// * `backend_id` - Unique identifier for this backend instance (optional, defaults to "todoist")
    /// * `backend_name` - Human-readable name for this backend (optional, defaults to "Todoist")
    pub fn new(
        api_token: String,
        backend_id: Option<String>,
        backend_name: Option<String>,
    ) -> Self {
        Self {
            wrapper: TodoistWrapper::new(api_token),
            backend_id: backend_id.unwrap_or_else(|| "todoist".to_string()),
            backend_name: backend_name.unwrap_or_else(|| "Todoist".to_string()),
        }
    }

    /// Convert backend CreateProjectArgs to Todoist API args
    fn convert_create_project_args(&self, args: &CreateProjectArgs) -> TodoistCreateProjectArgs {
        TodoistCreateProjectArgs {
            name: args.name.clone(),
            color: None,
            parent_id: args.parent_id.clone(),
            is_favorite: None,
            view_style: None,
        }
    }

    /// Convert backend CreateTaskArgs to Todoist API args
    fn convert_create_task_args(&self, args: &CreateTaskArgs) -> todoist_api::CreateTaskArgs {
        todoist_api::CreateTaskArgs {
            content: args.content.clone(),
            description: args.description.clone(),
            project_id: args.project_id.clone(),
            section_id: args.section_id.clone(),
            parent_id: args.parent_id.clone(),
            order: None,
            priority: args.priority,
            labels: args.labels.clone(),
            due_string: args.due_string.clone(),
            due_date: args.due_date.clone(),
            due_datetime: args.due_datetime.clone(),
            due_lang: None,
            deadline_date: args.deadline_date.clone(),
            deadline_lang: None,
            assignee_id: None,
            duration: args.duration.as_ref().map(|d| {
                // Parse duration string like "2h", "30m" into amount and unit
                if let Some(stripped) = d.strip_suffix('h') {
                    if let Ok(hours) = stripped.parse::<i32>() {
                        return hours;
                    }
                } else if let Some(stripped) = d.strip_suffix('m') {
                    if let Ok(minutes) = stripped.parse::<i32>() {
                        return minutes;
                    }
                }
                // Default to 30 minutes if parsing fails
                30
            }),
            duration_unit: args.duration.as_ref().map(|d| {
                if d.ends_with('h') {
                    "hour".to_string()
                } else if d.ends_with('m') {
                    "minute".to_string()
                } else {
                    "minute".to_string() // Default to minutes
                }
            }),
        }
    }

    /// Convert backend UpdateTaskArgs to Todoist API args
    fn convert_update_task_args(&self, args: &UpdateTaskArgs) -> todoist_api::UpdateTaskArgs {
        todoist_api::UpdateTaskArgs {
            content: args.content.clone(),
            description: args.description.clone(),
            labels: args.labels.clone(),
            priority: args.priority,
            due_string: args.due_string.clone(),
            due_date: args.due_date.clone(),
            due_datetime: args.due_datetime.clone(),
            due_lang: None,
            deadline_date: args.deadline_date.clone(),
            deadline_lang: None,
            assignee_id: None,
            duration: args.duration.as_ref().map(|d| {
                if let Some(stripped) = d.strip_suffix('h') {
                    if let Ok(hours) = stripped.parse::<i32>() {
                        return hours;
                    }
                } else if let Some(stripped) = d.strip_suffix('m') {
                    if let Ok(minutes) = stripped.parse::<i32>() {
                        return minutes;
                    }
                }
                30 // Default fallback
            }),
            duration_unit: args.duration.as_ref().map(|d| {
                if d.ends_with('h') {
                    "hour".to_string()
                } else {
                    "minute".to_string()
                }
            }),
        }
    }

    /// Convert backend CreateLabelArgs to Todoist API args
    fn convert_create_label_args(&self, args: &CreateLabelArgs) -> todoist_api::CreateLabelArgs {
        todoist_api::CreateLabelArgs {
            name: args.name.clone(),
            color: None,
            order: None,
            is_favorite: None,
        }
    }

    /// Convert backend UpdateLabelArgs to Todoist API args
    fn convert_update_label_args(&self, args: &UpdateLabelArgs) -> todoist_api::UpdateLabelArgs {
        todoist_api::UpdateLabelArgs {
            name: args.name.clone(),
            color: None,
            order: None,
            is_favorite: None,
        }
    }

    /// Convert backend UpdateProjectArgs to Todoist API args
    fn convert_update_project_args(&self, args: &UpdateProjectArgs) -> todoist_api::UpdateProjectArgs {
        todoist_api::UpdateProjectArgs {
            name: args.name.clone(),
            color: None,
            is_favorite: None,
            view_style: None,
        }
    }
}

#[async_trait]
impl Backend for TodoistBackend {
    async fn get_projects(&self) -> Result<Vec<todoist_api::Project>> {
        self.wrapper.get_projects().await.map_err(Into::into)
    }

    async fn get_tasks(&self) -> Result<Vec<todoist_api::Task>> {
        self.wrapper.get_tasks().await.map_err(Into::into)
    }

    async fn get_labels(&self) -> Result<Vec<todoist_api::Label>> {
        self.wrapper.get_labels().await.map_err(Into::into)
    }

    async fn get_sections(&self) -> Result<Vec<todoist_api::Section>> {
        self.wrapper.get_sections().await.map_err(Into::into)
    }

    async fn create_project(&self, args: &CreateProjectArgs) -> Result<todoist_api::Project> {
        let todoist_args = self.convert_create_project_args(args);
        self.wrapper.create_project(&todoist_args).await.map_err(Into::into)
    }

    async fn create_task(&self, args: &CreateTaskArgs) -> Result<todoist_api::Task> {
        let todoist_args = self.convert_create_task_args(args);
        self.wrapper.create_task(&todoist_args).await.map_err(Into::into)
    }

    async fn create_label(&self, args: &CreateLabelArgs) -> Result<todoist_api::Label> {
        let todoist_args = self.convert_create_label_args(args);
        self.wrapper.create_label(&todoist_args).await.map_err(Into::into)
    }

    async fn update_project(&self, id: &str, args: &UpdateProjectArgs) -> Result<todoist_api::Project> {
        let todoist_args = self.convert_update_project_args(args);
        self.wrapper.update_project(id, &todoist_args).await.map_err(Into::into)
    }

    async fn update_task(&self, id: &str, args: &UpdateTaskArgs) -> Result<todoist_api::Task> {
        let todoist_args = self.convert_update_task_args(args);
        self.wrapper.update_task(id, &todoist_args).await.map_err(Into::into)
    }

    async fn update_label(&self, id: &str, args: &UpdateLabelArgs) -> Result<todoist_api::Label> {
        let todoist_args = self.convert_update_label_args(args);
        self.wrapper.update_label(id, &todoist_args).await.map_err(Into::into)
    }

    async fn delete_project(&self, id: &str) -> Result<()> {
        self.wrapper.delete_project(id).await.map_err(Into::into)
    }

    async fn delete_task(&self, id: &str) -> Result<()> {
        self.wrapper.delete_task(id).await.map_err(Into::into)
    }

    async fn delete_label(&self, id: &str) -> Result<()> {
        self.wrapper.delete_label(id).await.map_err(Into::into)
    }

    async fn complete_task(&self, id: &str) -> Result<()> {
        self.wrapper.complete_task(id).await.map_err(Into::into)
    }

    async fn reopen_task(&self, id: &str) -> Result<()> {
        self.wrapper.reopen_task(id).await.map_err(Into::into)
    }
}

#[async_trait]
impl BackendProvider for TodoistBackend {
    fn backend_id(&self) -> &str {
        &self.backend_id
    }

    fn backend_name(&self) -> &str {
        &self.backend_name
    }

    fn backend_type(&self) -> &str {
        "todoist"
    }

    fn is_authenticated(&self) -> bool {
        // For Todoist, we assume authentication is valid if we have a token
        // The actual validation happens during API calls
        true
    }

    async fn test_connection(&self) -> Result<()> {
        // Test connection by making a lightweight API call
        self.wrapper.get_projects().await?;
        Ok(())
    }

    async fn get_status(&self) -> Result<BackendStatus> {
        let is_connected = self.test_connection().await.is_ok();
        let (sync_error, last_sync) = if is_connected {
            (None, Some(Utc::now()))
        } else {
            (Some("Connection test failed".to_string()), None)
        };

        Ok(BackendStatus {
            backend_id: self.backend_id.clone(),
            backend_name: self.backend_name.clone(),
            backend_type: self.backend_type().to_string(),
            is_authenticated: self.is_authenticated(),
            is_connected,
            last_sync,
            sync_error,
        })
    }
}
