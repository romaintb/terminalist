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

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Create a new App instance
    #[must_use]
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

    /// Get projects sorted with favorites first within their own hierarchical level
    #[must_use]
    pub fn get_sorted_projects(&self) -> Vec<(usize, &ProjectDisplay)> {
        let mut sorted_projects: Vec<_> = self.projects.iter().enumerate().collect();
        
        // Helper function to get the root project ID (top-level parent)
        fn get_root_project_id(project: &ProjectDisplay, projects: &[ProjectDisplay]) -> String {
            let mut current = project;
            while let Some(parent_id) = &current.parent_id {
                if let Some(parent) = projects.iter().find(|p| p.id == *parent_id) {
                    current = parent;
                } else {
                    break;
                }
            }
            current.id.clone()
        }
        
        // Helper function to get the immediate parent ID
        fn get_immediate_parent_id(project: &ProjectDisplay) -> Option<String> {
            project.parent_id.clone()
        }
        
        sorted_projects.sort_by(|(_a_idx, a_project), (_b_idx, b_project)| {
            // First, sort by root project to keep tree structures together
            let a_root = get_root_project_id(a_project, &self.projects);
            let b_root = get_root_project_id(b_project, &self.projects);
            let root_cmp = a_root.cmp(&b_root);
            if root_cmp != std::cmp::Ordering::Equal {
                return root_cmp;
            }
            
            // Same root, now sort by immediate parent to keep siblings together
            let a_parent = get_immediate_parent_id(a_project);
            let b_parent = get_immediate_parent_id(b_project);
            let parent_cmp = a_parent.cmp(&b_parent);
            if parent_cmp != std::cmp::Ordering::Equal {
                return parent_cmp;
            }
            
            // Same immediate parent (siblings), sort favorites first, then by name
            match (a_project.is_favorite, b_project.is_favorite) {
                (true, false) => std::cmp::Ordering::Less,    // a (favorite) comes before b (non-favorite)
                (false, true) => std::cmp::Ordering::Greater, // a (non-favorite) comes after b (favorite)
                _ => a_project.name.cmp(&b_project.name),     // Same favorite status, sort by name
            }
        });
        sorted_projects
    }

    /// Get the currently selected project from the sorted display order
    #[must_use]
    pub fn get_selected_project_from_sorted(&self) -> Option<&ProjectDisplay> {
        let sorted_projects = self.get_sorted_projects();
        let display_index = sorted_projects
            .iter()
            .position(|(original_idx, _)| *original_idx == self.selected_project_index)?;
        Some(sorted_projects[display_index].1)
    }

    /// Get the display index (position in sorted list) for the currently selected project
    #[must_use]
    pub fn get_selected_project_display_index(&self) -> Option<usize> {
        let sorted_projects = self.get_sorted_projects();
        sorted_projects
            .iter()
            .position(|(original_idx, _)| *original_idx == self.selected_project_index)
    }

    /// Select project by display index (position in sorted list)
    pub fn select_project_by_display_index(&mut self, display_index: usize) {
        let sorted_projects = self.get_sorted_projects();
        if let Some((original_index, _)) = sorted_projects.get(display_index) {
            self.selected_project_index = *original_index;
            self.project_list_state.select(Some(display_index));
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
                        self.error_message = Some(format!("Error loading tasks: {e}"));
                    }
                }
            }
            Err(e) => {
                self.error_message = Some(format!("Error loading projects: {e}"));
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
                        let a_score = if a.is_deleted { 2 } else { i32::from(a.is_completed) };
                        let b_score = if b.is_deleted { 2 } else { i32::from(b.is_completed) };
                        
                        // Sort by score (lower score = higher priority)
                        a_score.cmp(&b_score)
                    });
                    
                    self.tasks = sorted_tasks;
                    // Reset task selection to first task
                    self.selected_task_index = 0;
                    self.task_list_state.select(Some(0));
                }
                Err(e) => {
                    self.error_message = Some(format!("Error loading tasks: {e}"));
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    pub fn next_project(&mut self) {
        if !self.projects.is_empty() {
            let sorted_projects = self.get_sorted_projects();
            let current_display_index = self.get_selected_project_display_index().unwrap_or(0);
            let next_display_index = (current_display_index + 1) % sorted_projects.len();
            self.select_project_by_display_index(next_display_index);
        }
    }

    pub fn previous_project(&mut self) {
        if !self.projects.is_empty() {
            let sorted_projects = self.get_sorted_projects();
            let current_display_index = self.get_selected_project_display_index().unwrap_or(0);
            let prev_display_index = if current_display_index == 0 {
                sorted_projects.len() - 1
            } else {
                current_display_index - 1
            };
            self.select_project_by_display_index(prev_display_index);
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
                        self.error_message = Some(format!("Error reloading tasks: {e}"));
                    }
                }
                Err(e) => {
                    self.error_message = Some(format!("Error toggling task: {e}"));
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
                        self.error_message = Some(format!("Error reloading tasks: {e}"));
                    }
                }
                Err(e) => {
                    self.error_message = Some(format!("Error deleting task: {e}"));
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
                self.error_message = Some(format!("Sync error: {e}"));
            }
        }

        self.syncing = false;
    }
}
