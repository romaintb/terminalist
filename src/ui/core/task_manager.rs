use super::actions::{Action, NavigationCounts, SidebarSelection};
use super::ViewSnapshot;
use crate::constants::UI_LOADING_DATA_FROM_STORAGE;
use crate::sync::{SyncService, SyncStatus};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use uuid::Uuid;

pub type TaskId = u64;

#[derive(Debug)]
pub struct BackgroundTask {
    pub id: TaskId,
    pub handle: JoinHandle<anyhow::Result<TaskResult>>,
    pub description: String,
    pub blocks_input: bool,
    pub task_uuid: Option<Uuid>,
    pub started_at: std::time::Instant,
}

#[derive(Debug, Clone)]
pub enum TaskResult {
    SyncCompleted(SyncStatus),
    SyncFailed(String),
    TaskOperationCompleted(String),
    DataLoadCompleted(Box<ViewSnapshot>),
    SearchCompleted {
        query: String,
        results: Vec<crate::entities::task::Model>,
    },
    Other(String),
}

pub struct TaskManager {
    tasks: HashMap<TaskId, BackgroundTask>,
    next_task_id: TaskId,
    action_sender: mpsc::UnboundedSender<Action>,
}

impl TaskManager {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<Action>) {
        let (tx, rx) = mpsc::unbounded_channel();

        (
            Self {
                tasks: HashMap::new(),
                next_task_id: 1,
                action_sender: tx,
            },
            rx,
        )
    }

    /// Spawn a background sync operation
    pub fn spawn_sync(&mut self, sync_service: SyncService) -> TaskId {
        let task_id = self.next_task_id;
        self.next_task_id += 1;

        let action_sender = self.action_sender.clone();
        let description = "Background sync".to_string();

        let handle = tokio::spawn(async move {
            // Send sync started notification
            let _ = action_sender.send(Action::StartSync);

            match sync_service.force_sync().await {
                Ok(status) => {
                    let result = TaskResult::SyncCompleted(status.clone());
                    let _ = action_sender.send(Action::SyncCompleted(status));
                    Ok(result)
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    let result = TaskResult::SyncFailed(error_msg.clone());
                    let _ = action_sender.send(Action::SyncFailed(error_msg));
                    Ok(result)
                }
            }
        });

        let task = BackgroundTask {
            id: task_id,
            handle,
            description,
            blocks_input: false,
            task_uuid: None,
            started_at: std::time::Instant::now(),
        };

        self.tasks.insert(task_id, task);
        task_id
    }

    /// Spawn a background task operation (create, update, delete)
    pub fn spawn_task_operation<F, Fut>(&mut self, operation: F, description: String) -> TaskId
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = anyhow::Result<String>> + Send + 'static,
    {
        self.spawn_task_operation_with_input_policy(operation, description, true, None)
    }

    /// Spawn a background task operation while keeping keyboard input responsive.
    pub fn spawn_non_blocking_task_operation<F, Fut>(
        &mut self,
        task_uuid: Uuid,
        operation: F,
        description: String,
    ) -> TaskId
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = anyhow::Result<String>> + Send + 'static,
    {
        self.spawn_task_operation_with_input_policy(operation, description, false, Some(task_uuid))
    }

    fn spawn_task_operation_with_input_policy<F, Fut>(
        &mut self,
        operation: F,
        description: String,
        blocks_input: bool,
        task_uuid: Option<Uuid>,
    ) -> TaskId
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = anyhow::Result<String>> + Send + 'static,
    {
        let task_id = self.next_task_id;
        self.next_task_id += 1;

        let action_sender = self.action_sender.clone();
        let desc_clone = description.clone();
        let desc_for_task = description.clone();

        let handle = tokio::spawn(async move {
            match operation().await {
                Ok(message) => {
                    let result = TaskResult::TaskOperationCompleted(message.clone());
                    // Send refresh action to update UI with latest data from database
                    let _ = action_sender.send(Action::RefreshData);

                    // For project deletion, navigate to Today view to avoid empty selection
                    if desc_clone.starts_with("Delete project") {
                        let _ = action_sender.send(Action::NavigateToSidebar(SidebarSelection::Today));
                    }

                    Ok(result)
                }
                Err(e) => {
                    let error_msg = format!("Operation failed: {}", e);
                    let result = TaskResult::Other(error_msg.clone());
                    let _ = action_sender.send(Action::ShowDialog(crate::ui::core::actions::DialogType::Error(
                        error_msg,
                    )));
                    Ok(result)
                }
            }
        });

        let task = BackgroundTask {
            id: task_id,
            handle,
            description: desc_for_task,
            blocks_input,
            task_uuid,
            started_at: std::time::Instant::now(),
        };

        self.tasks.insert(task_id, task);
        task_id
    }

    /// Check for completed tasks and clean them up
    pub fn cleanup_finished_tasks(&mut self) -> Vec<(TaskId, anyhow::Result<TaskResult>)> {
        let mut completed = Vec::new();
        let mut to_remove = Vec::new();

        for (task_id, task) in &mut self.tasks {
            if task.handle.is_finished() {
                to_remove.push(*task_id);
            }
        }

        for task_id in to_remove {
            if let Some(_task) = self.tasks.remove(&task_id) {
                // Since the task is finished, we'll just mark it as completed
                // The actual result was already sent via the action channel
                let result = Ok(TaskResult::Other("Task completed".to_string()));
                completed.push((task_id, result));
            }
        }

        completed
    }

    /// Check if any sync tasks are currently running
    pub fn is_syncing(&self) -> bool {
        self.tasks.values().any(|task| task.description.contains("sync"))
    }

    pub fn has_blocking_work(&self) -> bool {
        self.tasks.values().any(|task| task.blocks_input)
    }

    pub fn has_pending_operation_for_task(&self, task_uuid: &Uuid) -> bool {
        self.tasks.values().any(|task| task.task_uuid.as_ref() == Some(task_uuid))
    }

    pub fn processing_description(&self) -> Option<String> {
        self.tasks
            .values()
            .find(|task| {
                task.description != UI_LOADING_DATA_FROM_STORAGE
                    && task.description != "Background sync"
                    && !task.description.starts_with("Searching tasks")
            })
            .map(|task| task.description.clone())
            .or_else(|| {
                self.tasks
                    .values()
                    .any(|task| task.description == UI_LOADING_DATA_FROM_STORAGE)
                    .then(|| "Refreshing tasks".to_string())
            })
    }

    /// Cancel all running tasks
    pub fn cancel_all_tasks(&mut self) {
        for (_, task) in self.tasks.drain() {
            task.handle.abort();
        }
    }

    /// Get the number of active tasks
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Spawn a background data loading operation
    pub fn spawn_data_load(
        &mut self,
        sync_service: SyncService,
        sidebar_selection: SidebarSelection,
        generation: u64,
        is_initial_load: bool,
    ) -> TaskId {
        let task_id = self.next_task_id;
        self.next_task_id += 1;

        let action_sender = self.action_sender.clone();
        let description = UI_LOADING_DATA_FROM_STORAGE.to_string();

        let handle = tokio::spawn(async move {
            match (
                sync_service.get_projects().await,
                sync_service.get_labels().await,
                sync_service.get_sections().await,
            ) {
                (Ok(projects), Ok(labels), Ok(sections)) => {
                    // Get tasks based on sidebar selection
                    let tasks = match &sidebar_selection {
                        SidebarSelection::Today => sync_service.get_tasks_for_today().await.unwrap_or_default(),
                        SidebarSelection::Tomorrow => sync_service.get_tasks_for_tomorrow().await.unwrap_or_default(),
                        SidebarSelection::Upcoming => sync_service.get_tasks_for_upcoming().await.unwrap_or_default(),
                        SidebarSelection::Project(project_uuid) => {
                            sync_service.get_tasks_for_project(project_uuid).await.unwrap_or_default()
                        }
                        SidebarSelection::Label(label_uuid) => {
                            sync_service.get_tasks_with_label(*label_uuid).await.unwrap_or_default()
                        }
                    };
                    let all_tasks = sync_service.get_all_tasks().await.unwrap_or_default();
                    let today = chrono::Local::now().date_naive();
                    let tomorrow = today + chrono::Duration::days(1);
                    let upcoming_end = today + chrono::Duration::days(90);
                    let mut navigation_counts = NavigationCounts::default();
                    for task in all_tasks.iter().filter(|task| !task.is_completed && !task.is_deleted) {
                        if let Some(due) = &task.due_date {
                            if let Ok(due) = crate::utils::datetime::parse_date(due) {
                                navigation_counts.today += usize::from(due <= today);
                                navigation_counts.tomorrow += usize::from(due == tomorrow);
                                navigation_counts.upcoming += usize::from(due <= upcoming_end);
                            }
                        }
                        *navigation_counts.projects.entry(task.project_uuid).or_default() += 1;
                    }
                    for label in &labels {
                        let count = sync_service
                            .get_tasks_with_label(label.uuid)
                            .await
                            .unwrap_or_default()
                            .iter()
                            .filter(|task| !task.is_completed && !task.is_deleted)
                            .count();
                        navigation_counts.labels.insert(label.uuid, count);
                    }

                    let snapshot = ViewSnapshot {
                        generation,
                        selection: sidebar_selection,
                        is_initial: is_initial_load,
                        projects: projects.clone(),
                        labels: labels.clone(),
                        sections: sections.clone(),
                        tasks: tasks.clone(),
                        all_tasks: all_tasks.clone(),
                        navigation_counts: navigation_counts.clone(),
                    };
                    let result = TaskResult::DataLoadCompleted(Box::new(snapshot.clone()));
                    let _ = action_sender.send(Action::DataLoaded(Box::new(snapshot)));
                    Ok(result)
                }
                (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => {
                    let error_msg = format!("Failed to load data: {}", e);
                    let _ = action_sender.send(Action::DataLoadFailed {
                        generation,
                        selection: sidebar_selection,
                        message: error_msg.clone(),
                    });
                    Ok(TaskResult::Other(error_msg))
                }
            }
        });

        let task = BackgroundTask {
            id: task_id,
            handle,
            description,
            blocks_input: true,
            task_uuid: None,
            started_at: std::time::Instant::now(),
        };

        self.tasks.insert(task_id, task);
        task_id
    }

    /// Spawn a background task search operation
    pub fn spawn_task_search(&mut self, sync_service: SyncService, query: String) -> TaskId {
        let task_id = self.next_task_id;
        self.next_task_id += 1;

        let action_sender = self.action_sender.clone();
        let description = format!("Searching tasks: '{}'", query);

        let handle = tokio::spawn(async move {
            match sync_service.search_tasks(&query).await {
                Ok(results) => {
                    let result = TaskResult::SearchCompleted {
                        query: query.clone(),
                        results: results.clone(),
                    };

                    let _ = action_sender.send(Action::SearchResultsLoaded { query, results });

                    Ok(result)
                }
                Err(e) => {
                    let error_msg = format!("Failed to search tasks: {}", e);
                    // Don't show error dialog for search failures, just log silently
                    Ok(TaskResult::Other(error_msg))
                }
            }
        });

        let task = BackgroundTask {
            id: task_id,
            handle,
            description,
            blocks_input: false,
            task_uuid: None,
            started_at: std::time::Instant::now(),
        };

        self.tasks.insert(task_id, task);
        task_id
    }
}

impl Drop for TaskManager {
    fn drop(&mut self) {
        // Cancel all tasks when the manager is dropped
        self.cancel_all_tasks();
    }
}
