use super::actions::{Action, LoadKind, SidebarSelection};
use crate::sync::SyncService;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub type TaskId = u64;

/// What a background task is doing. The UI asks about kinds rather than matching on a
/// human-readable description, which user text can imitate: a label or a search query
/// containing "sync" used to read as a sync in progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskKind {
    Sync,
    DataLoad,
    Search,
    Operation,
}

#[derive(Debug)]
pub struct BackgroundTask {
    pub handle: JoinHandle<()>,
    pub kind: TaskKind,
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

        let handle = tokio::spawn(async move {
            // Send sync started notification
            let _ = action_sender.send(Action::StartSync);

            let _ = match sync_service.force_sync().await {
                Ok(status) => action_sender.send(Action::SyncCompleted(status)),
                Err(e) => action_sender.send(Action::SyncFailed(e.to_string())),
            };
        });

        self.insert(task_id, handle, TaskKind::Sync);
        task_id
    }

    /// Spawn a background task operation (create, update, delete)
    /// `on_success` is sent after the operation succeeds, for the caller to steer the UI
    /// afterwards, such as leaving a project view whose project has just been deleted.
    pub fn spawn_task_operation<F, Fut>(&mut self, operation: F, on_success: Option<Action>) -> TaskId
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = anyhow::Result<String>> + Send + 'static,
    {
        let task_id = self.next_task_id;
        self.next_task_id += 1;

        let action_sender = self.action_sender.clone();

        let handle = tokio::spawn(async move {
            match operation().await {
                Ok(_message) => {
                    // Send refresh action to update UI with latest data from database
                    let _ = action_sender.send(Action::RefreshData);

                    if let Some(action) = on_success {
                        let _ = action_sender.send(action);
                    }
                }
                Err(e) => {
                    let _ = action_sender.send(Action::ShowDialog(crate::ui::core::actions::DialogType::Error(
                        format!("Operation failed: {e}"),
                    )));
                }
            }
        });

        self.insert(task_id, handle, TaskKind::Operation);
        task_id
    }

    /// Drop the bookkeeping for tasks that have finished, returning how many went.
    /// Results travel over the action channel, so there is nothing here to collect.
    pub fn cleanup_finished_tasks(&mut self) -> usize {
        let before = self.tasks.len();
        self.tasks.retain(|_, task| !task.handle.is_finished());
        before - self.tasks.len()
    }

    /// Check if any sync tasks are currently running
    pub fn is_syncing(&self) -> bool {
        self.tasks.values().any(|task| task.kind == TaskKind::Sync)
    }

    /// Cancel all running tasks
    pub fn cancel_all_tasks(&mut self) {
        for (_, task) in self.tasks.drain() {
            task.handle.abort();
        }
    }

    /// Spawn a background data loading operation
    pub fn spawn_data_load(
        &mut self,
        sync_service: SyncService,
        sidebar_selection: SidebarSelection,
        kind: LoadKind,
    ) -> TaskId {
        let task_id = self.next_task_id;
        self.next_task_id += 1;

        let action_sender = self.action_sender.clone();

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
                        SidebarSelection::Project(uuid) => {
                            sync_service.get_tasks_for_project(&uuid).await.unwrap_or_default()
                        }
                        SidebarSelection::Label(uuid) => {
                            sync_service.get_tasks_with_label(uuid).await.unwrap_or_default()
                        }
                    };

                    let _ = action_sender.send(Action::DataLoaded {
                        kind,
                        projects,
                        labels,
                        sections,
                        tasks,
                    });
                }
                (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => {
                    let _ = action_sender.send(Action::ShowDialog(crate::ui::core::actions::DialogType::Error(
                        format!("Failed to load data: {e}"),
                    )));
                }
            }
        });

        self.insert(task_id, handle, TaskKind::DataLoad);
        task_id
    }

    /// Spawn a background task search operation
    pub fn spawn_task_search(&mut self, sync_service: SyncService, query: String) -> TaskId {
        let task_id = self.next_task_id;
        self.next_task_id += 1;

        let action_sender = self.action_sender.clone();

        let handle = tokio::spawn(async move {
            match sync_service.search_tasks(&query).await {
                Ok(results) => {
                    let _ = action_sender.send(Action::SearchResultsLoaded { query, results });
                }
                // Search failures stay silent: no dialog, no toast.
                Err(e) => log::warn!("Failed to search tasks: {e}"),
            }
        });

        self.insert(task_id, handle, TaskKind::Search);
        task_id
    }

    fn insert(&mut self, id: TaskId, handle: JoinHandle<()>, kind: TaskKind) {
        self.tasks.insert(id, BackgroundTask { handle, kind });
    }
}

impl Drop for TaskManager {
    fn drop(&mut self) {
        // Cancel all tasks when the manager is dropped
        self.cancel_all_tasks();
    }
}
