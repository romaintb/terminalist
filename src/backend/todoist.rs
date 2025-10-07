//! Todoist backend implementation.

use async_trait::async_trait;
use crate::todoist::TodoistWrapper;
use super::{
    Backend, BackendError, BackendProject, BackendTask, BackendLabel, BackendSection,
    CreateProjectArgs, CreateTaskArgs, CreateLabelArgs,
    UpdateProjectArgs, UpdateTaskArgs, UpdateLabelArgs,
};

/// Todoist backend implementation.
pub struct TodoistBackend {
    wrapper: TodoistWrapper,
}

impl TodoistBackend {
    /// Create a new Todoist backend with the provided API token.
    pub fn new(api_token: String) -> Self {
        Self {
            wrapper: TodoistWrapper::new(api_token),
        }
    }

    // Helper: Transform Todoist API project → Backend project
    fn project_to_backend(api_project: &crate::todoist::Project) -> BackendProject {
        BackendProject {
            remote_id: api_project.id.clone(),
            name: api_project.name.clone(),
            color: api_project.color.clone(),
            is_favorite: api_project.is_favorite,
            is_inbox: api_project.is_inbox_project,
            order_index: api_project.order,
            parent_remote_id: api_project.parent_id.clone(),
        }
    }

    // Helper: Transform Todoist API task → Backend task
    fn task_to_backend(api_task: &crate::todoist::Task) -> BackendTask {
        BackendTask {
            remote_id: api_task.id.clone(),
            content: api_task.content.clone(),
            description: Some(api_task.description.clone()),
            project_remote_id: api_task.project_id.clone(),
            section_remote_id: api_task.section_id.clone(),
            parent_remote_id: api_task.parent_id.clone(),
            priority: api_task.priority,
            order_index: api_task.order,
            due_date: api_task.due.as_ref().map(|d| d.date.clone()),
            due_datetime: api_task.due.as_ref().and_then(|d| d.datetime.clone()),
            is_recurring: api_task.due.as_ref().map(|d| d.is_recurring).unwrap_or(false),
            deadline: None, // Todoist doesn't have deadline
            duration: api_task.duration.as_ref().map(|d| {
                format!("{} {}", d.amount, d.unit)
            }),
            is_completed: false, // Fetch operations don't include completed tasks
            labels: api_task.labels.clone(),
        }
    }

    // Helper: Transform Todoist API label → Backend label
    fn label_to_backend(api_label: &crate::todoist::Label) -> BackendLabel {
        BackendLabel {
            remote_id: api_label.id.clone(),
            name: api_label.name.clone(),
            color: api_label.color.clone(),
            order_index: api_label.order,
            is_favorite: api_label.is_favorite,
        }
    }

    // Helper: Transform Todoist API section → Backend section
    fn section_to_backend(api_section: &crate::todoist::Section) -> BackendSection {
        BackendSection {
            remote_id: api_section.id.clone(),
            name: api_section.name.clone(),
            project_remote_id: api_section.project_id.clone(),
            order_index: api_section.order,
        }
    }
}

#[async_trait]
impl Backend for TodoistBackend {
    fn backend_type(&self) -> &str {
        "todoist"
    }

    async fn fetch_projects(&self) -> Result<Vec<BackendProject>, BackendError> {
        let projects = self.wrapper.get_projects().await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        Ok(projects.iter().map(Self::project_to_backend).collect())
    }

    async fn fetch_tasks(&self) -> Result<Vec<BackendTask>, BackendError> {
        let tasks = self.wrapper.get_tasks().await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        Ok(tasks.iter().map(Self::task_to_backend).collect())
    }

    async fn fetch_labels(&self) -> Result<Vec<BackendLabel>, BackendError> {
        let labels = self.wrapper.get_labels().await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        Ok(labels.iter().map(Self::label_to_backend).collect())
    }

    async fn fetch_sections(&self) -> Result<Vec<BackendSection>, BackendError> {
        let sections = self.wrapper.get_sections().await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        Ok(sections.iter().map(Self::section_to_backend).collect())
    }

    async fn create_project(&self, args: CreateProjectArgs) -> Result<BackendProject, BackendError> {
        let todoist_args = crate::todoist::CreateProjectArgs {
            name: args.name,
            color: args.color,
            is_favorite: args.is_favorite,
            parent_id: args.parent_remote_id,
            view_style: None,
        };

        let project = self.wrapper.create_project(&todoist_args).await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        Ok(Self::project_to_backend(&project))
    }

    async fn update_project(&self, remote_id: &str, args: UpdateProjectArgs) -> Result<BackendProject, BackendError> {
        let todoist_args = crate::todoist::UpdateProjectArgs {
            name: args.name,
            color: args.color,
            is_favorite: args.is_favorite,
            view_style: None,
        };

        let project = self.wrapper.update_project(remote_id, &todoist_args).await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        Ok(Self::project_to_backend(&project))
    }

    async fn delete_project(&self, remote_id: &str) -> Result<(), BackendError> {
        self.wrapper.delete_project(remote_id).await
            .map_err(|e| BackendError::Network(e.to_string()))
    }

    async fn create_task(&self, args: CreateTaskArgs) -> Result<BackendTask, BackendError> {
        let todoist_args = crate::todoist::CreateTaskArgs {
            content: args.content,
            description: args.description,
            project_id: Some(args.project_remote_id),
            section_id: args.section_remote_id,
            parent_id: args.parent_remote_id,
            priority: args.priority,
            due_date: args.due_date,
            due_datetime: args.due_datetime,
            labels: Some(args.labels),
            duration: args.duration.as_ref().and_then(|d| {
                // CreateTaskArgs.duration is Option<i32> (just the amount)
                let parts: Vec<&str> = d.split_whitespace().collect();
                if !parts.is_empty() {
                    parts[0].parse().ok()
                } else {
                    None
                }
            }),
            ..Default::default()
        };

        let task = self.wrapper.create_task(&todoist_args).await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        Ok(Self::task_to_backend(&task))
    }

    async fn update_task(&self, remote_id: &str, args: UpdateTaskArgs) -> Result<BackendTask, BackendError> {
        let todoist_args = crate::todoist::UpdateTaskArgs {
            content: args.content,
            description: args.description,
            priority: args.priority,
            due_date: args.due_date,
            due_datetime: args.due_datetime,
            labels: args.labels,
            duration: args.duration.as_ref().and_then(|d| {
                // UpdateTaskArgs.duration is Option<i32> (just the amount)
                let parts: Vec<&str> = d.split_whitespace().collect();
                if !parts.is_empty() {
                    parts[0].parse().ok()
                } else {
                    None
                }
            }),
            ..Default::default()
        };

        let task = self.wrapper.update_task(remote_id, &todoist_args).await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        Ok(Self::task_to_backend(&task))
    }

    async fn delete_task(&self, remote_id: &str) -> Result<(), BackendError> {
        self.wrapper.delete_task(remote_id).await
            .map_err(|e| BackendError::Network(e.to_string()))
    }

    async fn complete_task(&self, remote_id: &str) -> Result<(), BackendError> {
        self.wrapper.complete_task(remote_id).await
            .map_err(|e| BackendError::Network(e.to_string()))
    }

    async fn reopen_task(&self, remote_id: &str) -> Result<(), BackendError> {
        self.wrapper.reopen_task(remote_id).await
            .map_err(|e| BackendError::Network(e.to_string()))
    }

    async fn create_label(&self, args: CreateLabelArgs) -> Result<BackendLabel, BackendError> {
        let todoist_args = crate::todoist::CreateLabelArgs {
            name: args.name,
            color: args.color,
            is_favorite: args.is_favorite,
            ..Default::default()
        };

        let label = self.wrapper.create_label(&todoist_args).await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        Ok(Self::label_to_backend(&label))
    }

    async fn update_label(&self, remote_id: &str, args: UpdateLabelArgs) -> Result<BackendLabel, BackendError> {
        let todoist_args = crate::todoist::UpdateLabelArgs {
            name: args.name,
            color: args.color,
            is_favorite: args.is_favorite,
            ..Default::default()
        };

        let label = self.wrapper.update_label(remote_id, &todoist_args).await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        Ok(Self::label_to_backend(&label))
    }

    async fn delete_label(&self, remote_id: &str) -> Result<(), BackendError> {
        self.wrapper.delete_label(remote_id).await
            .map_err(|e| BackendError::Network(e.to_string()))
    }
}
