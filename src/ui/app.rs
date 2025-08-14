//! Application state and business logic

use crate::sync::{SyncService, SyncStats, SyncStatus};
use crate::todoist::{ProjectDisplay, TaskDisplay};
use ratatui::widgets::ListState;

/// Application state
pub struct App {
    pub should_quit: bool,
    pub projects: Vec<ProjectDisplay>,
    pub tasks: Vec<TaskDisplay>,
    pub selected_project_index: usize,
    pub selected_task_index: usize,
    pub project_list_state: ListState,
    pub task_list_state: ListState,
    pub loading: bool,
    pub syncing: bool,
    pub completing_task: bool,
    pub deleting_task: bool,
    pub delete_confirmation: Option<String>, // Task ID to delete if confirmed
    pub error_message: Option<String>,
    pub sync_stats: Option<SyncStats>,
    pub last_sync_status: SyncStatus,
    pub show_help: bool, // Toggle for help panel
    pub help_scroll_offset: usize, // Scroll position for help panel
}

impl App {
    pub fn new() -> Self {
        let mut project_list_state = ListState::default();
        project_list_state.select(Some(0));

        let mut task_list_state = ListState::default();
        task_list_state.select(Some(0));

        Self {
            should_quit: false,
            projects: Vec::new(),
            tasks: Vec::new(),
            selected_project_index: 0,
            selected_task_index: 0,
            project_list_state,
            task_list_state,
            loading: true,
            syncing: false,
            completing_task: false,
            deleting_task: false,
            delete_confirmation: None,
            error_message: None,
            sync_stats: None,
            last_sync_status: SyncStatus::Idle,
            show_help: false,
            help_scroll_offset: 0,
        }
    }

    pub async fn load_local_data(&mut self, sync_service: &SyncService) {
        self.loading = true;
        self.error_message = None;

        // Remember the current project selection
        let current_project_id = self
            .projects
            .get(self.selected_project_index)
            .map(|p| p.id.clone());

        // Load projects from local storage (fast)
        match sync_service.get_projects().await {
            Ok(projects) => {
                self.projects = projects;
                if !self.projects.is_empty() {
                    // Try to maintain the current project selection
                    if let Some(current_id) = current_project_id {
                        // Find the previously selected project in the new list
                        if let Some(index) = self.projects.iter().position(|p| p.id == current_id) {
                            self.selected_project_index = index;
                            self.project_list_state.select(Some(index));
                        } else {
                            // If the project no longer exists, default to first project
                            self.selected_project_index = 0;
                            self.project_list_state.select(Some(0));
                        }
                    } else {
                        // No previous selection, default to first project
                        self.selected_project_index = 0;
                        self.project_list_state.select(Some(0));
                    }

                    // Load tasks for the selected project
                    if let Err(e) = self.load_tasks_for_selected_project(sync_service).await {
                        self.error_message = Some(format!("Error loading tasks: {}", e));
                    }
                }
            }
            Err(e) => {
                self.error_message = Some(format!("Error loading projects: {}", e));
            }
        }

        self.loading = false;
    }

    pub async fn load_tasks_for_selected_project(&mut self, sync_service: &SyncService) -> Result<(), anyhow::Error> {
        if let Some(project) = self.projects.get(self.selected_project_index) {
            match sync_service.get_tasks_for_project(&project.id).await {
                Ok(tasks) => {
                    // Sort tasks: pending first, then completed, then deleted
                    let mut sorted_tasks = tasks;
                    sorted_tasks.sort_by(|a, b| {
                        // Create priority scores: pending=0, completed=1, deleted=2
                        let a_score = if a.is_deleted { 2 } else if a.is_completed { 1 } else { 0 };
                        let b_score = if b.is_deleted { 2 } else if b.is_completed { 1 } else { 0 };
                        
                        // Sort by score (lower score = higher priority)
                        a_score.cmp(&b_score)
                    });
                    
                    self.tasks = sorted_tasks;
                    // Reset task selection to first task
                    if !self.tasks.is_empty() {
                        self.selected_task_index = 0;
                        self.task_list_state.select(Some(0));
                    } else {
                        self.selected_task_index = 0;
                        self.task_list_state.select(Some(0));
                    }
                }
                Err(e) => {
                    self.error_message = Some(format!("Error loading tasks: {}", e));
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    pub fn next_project(&mut self) {
        if !self.projects.is_empty() {
            self.selected_project_index = (self.selected_project_index + 1) % self.projects.len();
            self.project_list_state.select(Some(self.selected_project_index));
        }
    }

    pub fn previous_project(&mut self) {
        if !self.projects.is_empty() {
            self.selected_project_index = if self.selected_project_index == 0 {
                self.projects.len() - 1
            } else {
                self.selected_project_index - 1
            };
            self.project_list_state.select(Some(self.selected_project_index));
        }
    }

    pub fn next_task(&mut self) {
        if !self.tasks.is_empty() {
            self.selected_task_index = (self.selected_task_index + 1) % self.tasks.len();
            self.task_list_state.select(Some(self.selected_task_index));
        }
    }

    pub fn previous_task(&mut self) {
        if !self.tasks.is_empty() {
            self.selected_task_index = if self.selected_task_index == 0 {
                self.tasks.len() - 1
            } else {
                self.selected_task_index - 1
            };
            self.task_list_state.select(Some(self.selected_task_index));
        }
    }

    pub async fn toggle_selected_task(&mut self, sync_service: &SyncService) {
        if let Some(task) = self.tasks.get(self.selected_task_index) {
            self.completing_task = true;
            self.error_message = None;

            let result = if task.is_completed {
                sync_service.reopen_task(&task.id).await
            } else {
                sync_service.complete_task(&task.id).await
            };

            match result {
                Ok(()) => {
                    // Reload tasks to reflect the change
                    if let Err(e) = self.load_tasks_for_selected_project(sync_service).await {
                        self.error_message = Some(format!("Error reloading tasks: {}", e));
                    }
                }
                Err(e) => {
                    self.error_message = Some(format!("Error toggling task: {}", e));
                }
            }

            self.completing_task = false;
        }
    }

    pub async fn delete_selected_task(&mut self, sync_service: &SyncService) {
        if let Some(task) = self.tasks.get(self.selected_task_index) {
            self.deleting_task = true;
            self.error_message = None;

            match sync_service.delete_task(&task.id).await {
                Ok(()) => {
                    // Reload tasks to reflect the change
                    if let Err(e) = self.load_tasks_for_selected_project(sync_service).await {
                        self.error_message = Some(format!("Error reloading tasks: {}", e));
                    }
                }
                Err(e) => {
                    self.error_message = Some(format!("Error deleting task: {}", e));
                }
            }

            self.deleting_task = false;
            self.delete_confirmation = None;
        }
    }

    pub async fn force_clear_and_sync(&mut self, sync_service: &SyncService) {
        self.syncing = true;
        self.error_message = None;

        match sync_service.force_sync().await {
            Ok(stats) => {
                self.last_sync_status = stats;
                // Reload data after sync
                self.load_local_data(sync_service).await;
            }
            Err(e) => {
                self.error_message = Some(format!("Sync error: {}", e));
            }
        }

        self.syncing = false;
    }
}
