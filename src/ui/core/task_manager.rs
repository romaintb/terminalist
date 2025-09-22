use super::actions::{Action, SidebarSelection};
use crate::sync::{SyncService, SyncStatus};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub type TaskId = u64;

#[derive(Debug)]
pub struct BackgroundTask {
    pub id: TaskId,
    pub handle: JoinHandle<anyhow::Result<TaskResult>>,
    pub description: String,
    pub started_at: std::time::Instant,
}

#[derive(Debug, Clone)]
pub enum TaskResult {
    SyncCompleted(SyncStatus),
    SyncFailed(String),
    TaskOperationCompleted(String),
    DataLoadCompleted {
        projects: Vec<crate::todoist::ProjectDisplay>,
        labels: Vec<crate::todoist::LabelDisplay>,
        sections: Vec<crate::todoist::SectionDisplay>,
        tasks: Vec<crate::todoist::TaskDisplay>,
    },
    SearchCompleted {
        query: String,
        results: Vec<crate::todoist::TaskDisplay>,
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
        is_initial_load: bool,
    ) -> TaskId {
        let task_id = self.next_task_id;
        self.next_task_id += 1;

        let action_sender = self.action_sender.clone();
        let description = "Loading data from storage".to_string();

        let handle = tokio::spawn(async move {
            match (
                sync_service.get_projects().await,
                sync_service.get_labels().await,
                sync_service.get_sections().await,
            ) {
                (Ok(projects), Ok(labels), Ok(sections)) => {
                    // Get tasks based on sidebar selection
                    let tasks = match sidebar_selection {
                        SidebarSelection::Today => sync_service.get_tasks_for_today().await.unwrap_or_default(),
                        SidebarSelection::Tomorrow => sync_service.get_tasks_for_tomorrow().await.unwrap_or_default(),
                        SidebarSelection::Upcoming => sync_service.get_tasks_for_upcoming().await.unwrap_or_default(),
                        SidebarSelection::Project(index) => {
                            if let Some(project) = projects.get(index) {
                                sync_service.get_tasks_for_project(&project.id).await.unwrap_or_default()
                            } else {
                                Vec::new()
                            }
                        }
                        SidebarSelection::Label(index) => {
                            if let Some(label) = labels.get(index) {
                                sync_service.get_tasks_with_label(&label.name).await.unwrap_or_default()
                            } else {
                                Vec::new()
                            }
                        }
                    };

                    let result = TaskResult::DataLoadCompleted {
                        projects: projects.clone(),
                        labels: labels.clone(),
                        sections: sections.clone(),
                        tasks: tasks.clone(),
                    };

                    let action = if is_initial_load {
                        Action::InitialDataLoaded {
                            projects,
                            labels,
                            sections,
                            tasks,
                        }
                    } else {
                        Action::DataLoaded {
                            projects,
                            labels,
                            sections,
                            tasks,
                        }
                    };
                    let _ = action_sender.send(action);

                    Ok(result)
                }
                (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => {
                    let error_msg = format!("Failed to load data: {}", e);
                    let _ = action_sender.send(Action::ShowDialog(crate::ui::core::actions::DialogType::Error(
                        error_msg.clone(),
                    )));
                    Ok(TaskResult::Other(error_msg))
                }
            }
        });

        let task = BackgroundTask {
            id: task_id,
            handle,
            description,
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

// Utility trait for future extensions
#[allow(dead_code)]
trait BackgroundTaskExt {
    fn elapsed(&self) -> std::time::Duration;
    fn is_long_running(&self) -> bool;
}

impl BackgroundTaskExt for BackgroundTask {
    fn elapsed(&self) -> std::time::Duration {
        self.started_at.elapsed()
    }

    fn is_long_running(&self) -> bool {
        self.elapsed() > std::time::Duration::from_secs(30)
    }
}
