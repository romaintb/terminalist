//! Todoist backend implementation.

use super::{
    Backend, BackendError, BackendLabel, BackendProject, BackendSection, BackendTask, CreateLabelArgs,
    CreateProjectArgs, CreateTaskArgs, UpdateLabelArgs, UpdateProjectArgs, UpdateTaskArgs,
};
use crate::todoist::TodoistWrapper;
use async_trait::async_trait;
use serde::Deserialize;

#[derive(Deserialize)]
struct CompletedTasksPage {
    #[serde(alias = "results")]
    items: Vec<crate::todoist::Task>,
    next_cursor: Option<String>,
}

/// Todoist backend implementation.
pub struct TodoistBackend {
    wrapper: TodoistWrapper,
    api_token: String,
    client: reqwest::Client,
}

impl TodoistBackend {
    /// Create a new Todoist backend with the provided API token.
    pub fn new(api_token: String) -> Self {
        Self {
            wrapper: TodoistWrapper::new(api_token.clone()),
            api_token,
            client: reqwest::Client::new(),
        }
    }

    // Helper: Transform Todoist API project → Backend project
    fn project_to_backend(api_project: &crate::todoist::Project) -> BackendProject {
        BackendProject {
            remote_id: api_project.id.clone(),
            name: api_project.name.clone(),
            is_favorite: api_project.is_favorite,
            is_inbox: api_project.inbox_project,
            order_index: 0, // order field removed from API v1
            parent_remote_id: api_project.parent_id.clone(),
        }
    }

    // Helper: Transform Todoist API task → Backend task
    fn task_to_backend(api_task: &crate::todoist::Task) -> BackendTask {
        let due_datetime = api_task.due.as_ref().and_then(|due| {
            due.datetime
                .clone()
                .or_else(|| due.date.contains('T').then(|| due.date.clone()))
        });
        let due_date = api_task
            .due
            .as_ref()
            .and_then(|due| due.date.split('T').next().map(str::to_string));
        BackendTask {
            remote_id: api_task.id.clone(),
            content: api_task.content.clone(),
            description: Some(api_task.description.clone()),
            project_remote_id: api_task.project_id.clone(),
            section_remote_id: api_task.section_id.clone(),
            parent_remote_id: api_task.parent_id.clone(),
            priority: api_task.priority,
            order_index: 0, // order field removed from API v1
            due_date,
            due_datetime,
            is_recurring: api_task.due.as_ref().map(|d| d.is_recurring).unwrap_or(false),
            deadline: None, // Todoist doesn't have deadline
            duration: api_task.duration.as_ref().map(|d| format!("{} {}", d.amount, d.unit)),
            is_completed: api_task.checked || api_task.completed_at.is_some(),
            completed_at: api_task.completed_at.clone(),
            labels: api_task.labels.clone(),
        }
    }

    // Helper: Transform Todoist API label → Backend label
    fn label_to_backend(api_label: &crate::todoist::Label) -> BackendLabel {
        BackendLabel {
            remote_id: api_label.id.clone(),
            name: api_label.name.clone(),
            order_index: api_label.order.unwrap_or(0),
            is_favorite: api_label.is_favorite,
        }
    }

    // Helper: Transform Todoist API section → Backend section
    fn section_to_backend(api_section: &crate::todoist::Section) -> BackendSection {
        BackendSection {
            remote_id: api_section.id.clone(),
            name: api_section.name.clone(),
            project_remote_id: api_section.project_id.clone(),
            order_index: api_section.section_order,
        }
    }

    fn task_create_args_to_todoist(args: CreateTaskArgs) -> crate::todoist::CreateTaskArgs {
        crate::todoist::CreateTaskArgs {
            content: args.content,
            description: args.description,
            project_id: args.project_remote_id,
            section_id: args.section_remote_id,
            parent_id: args.parent_remote_id,
            priority: args.priority,
            due_date: args.due_date,
            due_datetime: args.due_datetime,
            labels: Some(args.labels),
            duration: args
                .duration
                .as_deref()
                .and_then(|duration| duration.split_whitespace().next())
                .and_then(|amount| amount.parse().ok()),
            ..Default::default()
        }
    }
}

#[async_trait]
impl Backend for TodoistBackend {
    fn backend_type(&self) -> &str {
        "todoist"
    }

    async fn fetch_projects(&self) -> Result<Vec<BackendProject>, BackendError> {
        let mut all_projects = Vec::new();
        let mut cursor: Option<String> = None;

        // Fetch all pages with limit=200
        loop {
            let response = self
                .wrapper
                .get_projects(Some(200), cursor.clone())
                .await
                .map_err(|e| BackendError::Network(e.to_string()))?;

            all_projects.extend(response.results.iter().map(Self::project_to_backend));

            // Check if there are more pages
            if response.next_cursor.is_none() {
                break;
            }
            cursor = response.next_cursor;
        }

        Ok(all_projects)
    }

    async fn fetch_tasks(&self) -> Result<Vec<BackendTask>, BackendError> {
        let mut all_tasks = Vec::new();
        let mut cursor: Option<String> = None;

        // Fetch all pages with limit=200
        loop {
            let response = self
                .wrapper
                .get_tasks(Some(200), cursor.clone())
                .await
                .map_err(|e| BackendError::Network(e.to_string()))?;

            all_tasks.extend(response.results.iter().map(Self::task_to_backend));

            // Check if there are more pages
            if response.next_cursor.is_none() {
                break;
            }
            cursor = response.next_cursor;
        }

        Ok(all_tasks)
    }

    async fn fetch_completed_tasks(&self, since: &str, until: &str) -> Result<Vec<BackendTask>, BackendError> {
        let mut all_tasks = Vec::new();
        let mut cursor = None;

        loop {
            let mut request = self
                .client
                .get("https://api.todoist.com/api/v1/tasks/completed/by_completion_date")
                .bearer_auth(&self.api_token)
                .query(&[("since", since), ("until", until), ("limit", "200")]);
            if let Some(cursor) = &cursor {
                request = request.query(&[("cursor", cursor)]);
            }
            let response = request.send().await.map_err(|e| BackendError::Network(e.to_string()))?;
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(BackendError::Other(format!("Todoist returned {status}: {body}")));
            }
            let response = response
                .json::<CompletedTasksPage>()
                .await
                .map_err(|e| BackendError::InvalidData(e.to_string()))?;

            all_tasks.extend(response.items.iter().map(Self::task_to_backend));
            if response.next_cursor.is_none() {
                break;
            }
            cursor = response.next_cursor;
        }

        Ok(all_tasks)
    }

    async fn fetch_labels(&self) -> Result<Vec<BackendLabel>, BackendError> {
        let mut all_labels = Vec::new();
        let mut cursor: Option<String> = None;

        // Fetch all pages with limit=200
        loop {
            let response = self
                .wrapper
                .get_labels(Some(200), cursor.clone())
                .await
                .map_err(|e| BackendError::Network(e.to_string()))?;

            all_labels.extend(response.results.iter().map(Self::label_to_backend));

            // Check if there are more pages
            if response.next_cursor.is_none() {
                break;
            }
            cursor = response.next_cursor;
        }

        Ok(all_labels)
    }

    async fn fetch_sections(&self) -> Result<Vec<BackendSection>, BackendError> {
        let mut all_sections = Vec::new();
        let mut cursor: Option<String> = None;

        // Fetch all pages with limit=200
        loop {
            let response = self
                .wrapper
                .get_sections(Some(200), cursor.clone())
                .await
                .map_err(|e| BackendError::Network(e.to_string()))?;

            all_sections.extend(response.results.iter().map(Self::section_to_backend));

            // Check if there are more pages
            if response.next_cursor.is_none() {
                break;
            }
            cursor = response.next_cursor;
        }

        Ok(all_sections)
    }

    async fn create_project(&self, args: CreateProjectArgs) -> Result<BackendProject, BackendError> {
        let todoist_args = crate::todoist::CreateProjectArgs {
            name: args.name,
            color: None,
            is_favorite: args.is_favorite,
            parent_id: args.parent_remote_id,
            view_style: None,
        };

        let project = self
            .wrapper
            .create_project(&todoist_args)
            .await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        Ok(Self::project_to_backend(&project))
    }

    async fn update_project(&self, remote_id: &str, args: UpdateProjectArgs) -> Result<BackendProject, BackendError> {
        let todoist_args = crate::todoist::UpdateProjectArgs {
            name: args.name,
            color: None,
            is_favorite: args.is_favorite,
            view_style: None,
        };

        let project = self
            .wrapper
            .update_project(remote_id, &todoist_args)
            .await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        Ok(Self::project_to_backend(&project))
    }

    async fn delete_project(&self, remote_id: &str) -> Result<(), BackendError> {
        self.wrapper
            .delete_project(remote_id)
            .await
            .map_err(|e| BackendError::Network(e.to_string()))
    }

    async fn create_task(&self, args: CreateTaskArgs) -> Result<BackendTask, BackendError> {
        let todoist_args = Self::task_create_args_to_todoist(args);

        let task = self
            .wrapper
            .create_task(&todoist_args)
            .await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        Ok(Self::task_to_backend(&task))
    }

    async fn update_task(&self, remote_id: &str, args: UpdateTaskArgs) -> Result<BackendTask, BackendError> {
        if args.clear_due_date {
            let response = self
                .client
                .post(format!("https://api.todoist.com/api/v1/tasks/{remote_id}"))
                .bearer_auth(&self.api_token)
                .json(&serde_json::json!({ "due_string": "no date" }))
                .send()
                .await
                .map_err(|error| BackendError::Network(error.to_string()))?;
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(BackendError::Other(format!("Todoist returned {status}: {body}")));
            }
            let task = response
                .json::<crate::todoist::Task>()
                .await
                .map_err(|error| BackendError::InvalidData(error.to_string()))?;
            return Ok(Self::task_to_backend(&task));
        }

        let todoist_args = crate::todoist::UpdateTaskArgs {
            content: args.content,
            description: args.description,
            priority: args.priority,
            due_string: None,
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

        let task = self
            .wrapper
            .update_task(remote_id, &todoist_args)
            .await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        Ok(Self::task_to_backend(&task))
    }

    async fn delete_task(&self, remote_id: &str) -> Result<(), BackendError> {
        self.wrapper
            .delete_task(remote_id)
            .await
            .map_err(|e| BackendError::Network(e.to_string()))
    }

    async fn complete_task(&self, remote_id: &str) -> Result<(), BackendError> {
        self.wrapper
            .complete_task(remote_id)
            .await
            .map_err(|e| BackendError::Network(e.to_string()))
    }

    async fn reopen_task(&self, remote_id: &str) -> Result<(), BackendError> {
        self.wrapper
            .reopen_task(remote_id)
            .await
            .map_err(|e| BackendError::Network(e.to_string()))
    }

    async fn create_label(&self, args: CreateLabelArgs) -> Result<BackendLabel, BackendError> {
        let todoist_args = crate::todoist::CreateLabelArgs {
            name: args.name,
            color: None,
            is_favorite: args.is_favorite,
            ..Default::default()
        };

        let label = self
            .wrapper
            .create_label(&todoist_args)
            .await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        Ok(Self::label_to_backend(&label))
    }

    async fn update_label(&self, remote_id: &str, args: UpdateLabelArgs) -> Result<BackendLabel, BackendError> {
        let todoist_args = crate::todoist::UpdateLabelArgs {
            name: args.name,
            color: None,
            is_favorite: args.is_favorite,
            ..Default::default()
        };

        let label = self
            .wrapper
            .update_label(remote_id, &todoist_args)
            .await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        Ok(Self::label_to_backend(&label))
    }

    async fn delete_label(&self, remote_id: &str) -> Result<(), BackendError> {
        self.wrapper
            .delete_label(remote_id)
            .await
            .map_err(|e| BackendError::Network(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creating_an_inbox_task_omits_the_project_id() {
        let args = CreateTaskArgs {
            content: "Inbox task".to_string(),
            description: None,
            project_remote_id: None,
            section_remote_id: None,
            parent_remote_id: None,
            priority: None,
            due_date: None,
            due_datetime: None,
            duration: None,
            labels: Vec::new(),
        };

        let todoist_args = TodoistBackend::task_create_args_to_todoist(args);

        assert_eq!(todoist_args.project_id, None);
    }

    #[test]
    fn completed_task_uses_todoists_completion_timestamp() {
        let api_task: crate::todoist::Task = serde_json::from_value(serde_json::json!({
            "id": "completed-task",
            "user_id": "user",
            "content": "Exercise",
            "description": "",
            "project_id": "inbox",
            "section_id": null,
            "parent_id": null,
            "added_by_uid": "user",
            "assigned_by_uid": null,
            "responsible_uid": null,
            "labels": [],
            "deadline": null,
            "duration": null,
            "checked": true,
            "is_deleted": false,
            "added_at": "2026-07-18T10:00:00Z",
            "completed_at": "2026-07-18T14:30:00Z",
            "completed_by_uid": "user",
            "updated_at": "2026-07-18T14:30:00Z",
            "due": {
                "date": "2026-07-18",
                "string": "today",
                "lang": "en",
                "is_recurring": false
            },
            "priority": 1,
            "child_order": 0,
            "note_count": 0,
            "day_order": 0,
            "is_collapsed": false
        }))
        .unwrap();

        let task = TodoistBackend::task_to_backend(&api_task);
        assert!(task.is_completed);
        assert_eq!(task.completed_at.as_deref(), Some("2026-07-18T14:30:00Z"));
    }

    #[test]
    fn completed_endpoint_uses_items_response_shape() {
        let response: CompletedTasksPage = serde_json::from_value(serde_json::json!({
            "items": [],
            "next_cursor": null
        }))
        .unwrap();

        assert!(response.items.is_empty());
        assert!(response.next_cursor.is_none());
    }
}
