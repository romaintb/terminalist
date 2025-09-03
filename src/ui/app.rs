//! Application state and business logic

use crate::debug_logger::DebugLogger;
use crate::icons::IconService;
use crate::sync::{SyncService, SyncStats, SyncStatus};
use crate::todoist::{LabelDisplay, ProjectDisplay, SectionDisplay, TaskDisplay};
use ratatui::widgets::ListState;
use tokio::task::JoinHandle;

/// Represents the currently selected item in the sidebar
#[derive(Debug, Clone, PartialEq)]
pub enum SidebarSelection {
    Today,          // Today view (special view)
    Tomorrow,       // Tomorrow view (special view)
    Label(usize),   // Index into labels vector
    Project(usize), // Index into projects vector
}

/// Application state
pub struct App {
    pub should_quit: bool,
    pub projects: Vec<ProjectDisplay>,
    pub tasks: Vec<TaskDisplay>,
    pub labels: Vec<LabelDisplay>,
    pub sections: Vec<SectionDisplay>,
    pub sidebar_selection: SidebarSelection,
    pub selected_task_index: usize,

    pub task_list_state: ListState,
    pub loading: bool,
    pub syncing: bool,
    pub completing_task: bool,
    pub deleting_task: bool,
    pub delete_confirmation: Option<String>, // Task ID to delete if confirmed
    pub error_message: Option<String>,
    pub info_message: Option<String>,
    pub sync_stats: Option<SyncStats>,
    pub last_sync_status: SyncStatus,
    pub show_help: bool,           // Toggle for help panel
    pub help_scroll_offset: usize, // Scroll position for help panel
    // Background sync task handle (if a sync is in progress)
    pub sync_task: Option<JoinHandle<anyhow::Result<crate::sync::SyncStatus>>>,
    // Project management
    pub creating_project: bool,
    pub new_project_name: String,
    pub new_project_parent_id: Option<String>,
    pub delete_project_confirmation: Option<String>, // Project ID to delete if confirmed
    pub editing_project: bool,
    pub edit_project_name: String,
    pub edit_project_id: Option<String>,
    // Label management
    pub creating_label: bool,
    pub new_label_name: String,
    pub editing_label: bool,
    pub edit_label_name: String,
    pub edit_label_id: Option<String>,
    pub delete_label_confirmation: Option<String>, // Label ID to delete if confirmed
    // Task management
    pub creating_task: bool,
    pub new_task_content: String,
    pub new_task_project_id: Option<String>,
    pub editing_task: bool,
    pub edit_task_content: String,
    pub edit_task_id: Option<String>,
    // Icons
    pub icons: IconService,
    // Debug logging
    pub debug_logger: DebugLogger,
    pub show_debug_modal: bool,
    pub debug_scroll_offset: usize,
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
        // Selection will be set when data is loaded

        let mut task_list_state = ListState::default();
        task_list_state.select(Some(0));

        Self {
            should_quit: false,
            projects: Vec::new(),
            tasks: Vec::new(),
            labels: Vec::new(),
            sections: Vec::new(),
            sidebar_selection: SidebarSelection::Today, // Default to Today view
            selected_task_index: 0,
            task_list_state,
            loading: true,
            syncing: false,
            completing_task: false,
            deleting_task: false,
            delete_confirmation: None,
            error_message: None,
            info_message: None,
            sync_stats: None,
            last_sync_status: SyncStatus::Idle,
            show_help: false,
            help_scroll_offset: 0,
            sync_task: None,
            // Project management
            creating_project: false,
            new_project_name: String::new(),
            new_project_parent_id: None,
            delete_project_confirmation: None,
            editing_project: false,
            edit_project_name: String::new(),
            edit_project_id: None,
            // Label management
            creating_label: false,
            new_label_name: String::new(),
            editing_label: false,
            edit_label_name: String::new(),
            edit_label_id: None,
            delete_label_confirmation: None,
            // Task management
            creating_task: false,
            new_task_content: String::new(),
            new_task_project_id: None,
            editing_task: false,
            edit_task_content: String::new(),
            edit_task_id: None,
            // Icons
            icons: IconService::default(),
            // Debug logging
            debug_logger: DebugLogger::new(),
            show_debug_modal: false,
            debug_scroll_offset: 0,
        }
    }

    /// Add a debug log entry
    pub fn add_debug_log(&mut self, message: String) {
        self.debug_logger.log(message);
    }

    /// Get the list position for the currently selected task (accounting for section headers)
    fn get_task_list_position(&self) -> Option<usize> {
        match &self.sidebar_selection {
            SidebarSelection::Today => {
                // For Today view, we need to account for section headers
                let now = chrono::Utc::now().date_naive();

                // Separate tasks into overdue and today
                let mut overdue_tasks = Vec::new();
                let mut today_tasks = Vec::new();

                for task in &self.tasks {
                    if let Some(due_date_str) = &task.due {
                        if let Ok(due_date) = chrono::NaiveDate::parse_from_str(due_date_str, "%Y-%m-%d") {
                            if due_date < now {
                                overdue_tasks.push(task);
                            } else if due_date == now {
                                today_tasks.push(task);
                            }
                        }
                    }
                }

                let mut list_position = 0;
                let mut current_task_index = 0;

                // Count overdue section
                if !overdue_tasks.is_empty() {
                    // Section header
                    list_position += 1;

                    if self.selected_task_index < current_task_index + overdue_tasks.len() {
                        return Some(list_position + (self.selected_task_index - current_task_index));
                    }
                    list_position += overdue_tasks.len();
                    current_task_index += overdue_tasks.len();

                    // Separator if we have both sections
                    if !today_tasks.is_empty() {
                        list_position += 1;
                    }
                }

                // Count today section
                if !today_tasks.is_empty() {
                    // Section header
                    list_position += 1;

                    if self.selected_task_index < current_task_index + today_tasks.len() {
                        return Some(list_position + (self.selected_task_index - current_task_index));
                    }
                    list_position += today_tasks.len();
                    current_task_index += today_tasks.len();
                }

                // If no tasks match the date filtering, show all tasks (fallback for debugging)
                if overdue_tasks.is_empty() && today_tasks.is_empty() && !self.tasks.is_empty() {
                    // Section header
                    list_position += 1;

                    if self.selected_task_index < current_task_index + self.tasks.len() {
                        return Some(list_position + (self.selected_task_index - current_task_index));
                    }
                }

                None
            }
            SidebarSelection::Tomorrow => {
                // For Tomorrow view, we just need to account for the section header
                if !self.tasks.is_empty() {
                    // Section header
                    let list_position = 1;
                    if self.selected_task_index < self.tasks.len() {
                        return Some(list_position + self.selected_task_index);
                    }
                }
                None
            }
            SidebarSelection::Project(index) => {
                let current_project = self.projects.get(*index).map(|p| &p.id);
                if let Some(project_id) = current_project {
                    // Get sections for the current project
                    let project_sections: Vec<_> = self
                        .sections
                        .iter()
                        .filter(|section| section.project_id == *project_id)
                        .collect();

                    // Group tasks by section
                    let mut tasks_by_section: std::collections::HashMap<Option<String>, Vec<&TaskDisplay>> =
                        std::collections::HashMap::new();

                    for task in &self.tasks {
                        tasks_by_section
                            .entry(task.section_id.clone())
                            .or_default()
                            .push(task);
                    }

                    let mut list_position = 0;
                    let mut current_task_index = 0;

                    // Count tasks without sections first
                    if let Some(tasks_without_section) = tasks_by_section.get(&None) {
                        if self.selected_task_index < current_task_index + tasks_without_section.len() {
                            return Some(list_position + (self.selected_task_index - current_task_index));
                        }
                        list_position += tasks_without_section.len();
                        current_task_index += tasks_without_section.len();
                    }

                    // Count sections and their tasks
                    for (section_index, section) in project_sections.iter().enumerate() {
                        // Add section header positions (blank, name, separator)
                        let section_header_count = if list_position > 0 || section_index > 0 { 3 } else { 2 };
                        list_position += section_header_count;

                        // Check if task is in this section
                        if let Some(section_tasks) = tasks_by_section.get(&Some(section.id.clone())) {
                            if self.selected_task_index < current_task_index + section_tasks.len() {
                                return Some(list_position + (self.selected_task_index - current_task_index));
                            }
                            list_position += section_tasks.len();
                            current_task_index += section_tasks.len();
                        }
                    }

                    None
                } else {
                    // No sections, task index equals list position
                    Some(self.selected_task_index)
                }
            }
            SidebarSelection::Label(_) => {
                // For label view, no section headers, task index equals list position
                Some(self.selected_task_index)
            }
        }
    }

    /// Get the currently selected project (if a project is selected)
    #[must_use]
    pub fn get_selected_project(&self) -> Option<&ProjectDisplay> {
        match &self.sidebar_selection {
            SidebarSelection::Project(index) => self.projects.get(*index),
            SidebarSelection::Label(_) => None,
            SidebarSelection::Today => None,
            SidebarSelection::Tomorrow => None,
        }
    }

    /// Get the currently selected label (if a label is selected)
    #[must_use]
    pub fn get_selected_label(&self) -> Option<&LabelDisplay> {
        match &self.sidebar_selection {
            SidebarSelection::Label(index) => self.labels.get(*index),
            SidebarSelection::Project(_) => None,
            SidebarSelection::Today => None,
            SidebarSelection::Tomorrow => None,
        }
    }

    /// Navigate to the next item in the sidebar
    pub fn next_sidebar_item(&mut self) {
        match &self.sidebar_selection {
            SidebarSelection::Today => {
                // Move to Tomorrow
                self.sidebar_selection = SidebarSelection::Tomorrow;
            }
            SidebarSelection::Tomorrow => {
                if !self.labels.is_empty() {
                    // Move to first label
                    self.sidebar_selection = SidebarSelection::Label(0);
                } else if !self.projects.is_empty() {
                    // Move to first project (use first in sorted order)
                    let sorted_projects = self.get_sorted_projects();
                    if let Some((original_index, _)) = sorted_projects.first() {
                        self.sidebar_selection = SidebarSelection::Project(*original_index);
                    }
                } else {
                    // Wrap to Tomorrow
                    self.sidebar_selection = SidebarSelection::Tomorrow;
                }
            }
            SidebarSelection::Label(index) => {
                let next_index = index + 1;
                if next_index < self.labels.len() {
                    // Move to next label
                    self.sidebar_selection = SidebarSelection::Label(next_index);
                } else if !self.projects.is_empty() {
                    // Move to first project (use first in sorted order)
                    let sorted_projects = self.get_sorted_projects();
                    if let Some((original_index, _)) = sorted_projects.first() {
                        self.sidebar_selection = SidebarSelection::Project(*original_index);
                    }
                } else {
                    // Wrap to Tomorrow
                    self.sidebar_selection = SidebarSelection::Tomorrow;
                }
            }
            SidebarSelection::Project(index) => {
                let sorted_projects = self.get_sorted_projects();
                if let Some(current_sorted_index) = sorted_projects
                    .iter()
                    .position(|(orig_idx, _)| orig_idx == index)
                {
                    let next_sorted_index = current_sorted_index + 1;
                    if next_sorted_index < sorted_projects.len() {
                        // Move to next project
                        if let Some((original_index, _)) = sorted_projects.get(next_sorted_index) {
                            self.sidebar_selection = SidebarSelection::Project(*original_index);
                        }
                    } else {
                        // Wrap to Tomorrow
                        self.sidebar_selection = SidebarSelection::Tomorrow;
                    }
                }
            }
        }
    }

    /// Navigate to the previous item in the sidebar
    pub fn previous_sidebar_item(&mut self) {
        match &self.sidebar_selection {
            SidebarSelection::Today => {
                // From Today, wrap to last project or last label
                if !self.projects.is_empty() {
                    let sorted_projects = self.get_sorted_projects();
                    if let Some((original_index, _)) = sorted_projects.last() {
                        self.sidebar_selection = SidebarSelection::Project(*original_index);
                    }
                } else if !self.labels.is_empty() {
                    self.sidebar_selection = SidebarSelection::Label(self.labels.len() - 1);
                } else {
                    // Wrap to Tomorrow
                    self.sidebar_selection = SidebarSelection::Tomorrow;
                }
            }
            SidebarSelection::Tomorrow => {
                // Move to Today
                self.sidebar_selection = SidebarSelection::Today;
            }
            SidebarSelection::Label(index) => {
                if *index > 0 {
                    // Move to previous label
                    self.sidebar_selection = SidebarSelection::Label(index - 1);
                } else {
                    // Wrap to Tomorrow
                    self.sidebar_selection = SidebarSelection::Tomorrow;
                }
            }
            SidebarSelection::Project(index) => {
                let sorted_projects = self.get_sorted_projects();
                if let Some(current_sorted_index) = sorted_projects
                    .iter()
                    .position(|(orig_idx, _)| orig_idx == index)
                {
                    if current_sorted_index > 0 {
                        // Move to previous project
                        if let Some((original_index, _)) = sorted_projects.get(current_sorted_index - 1) {
                            self.sidebar_selection = SidebarSelection::Project(*original_index);
                        }
                    } else if !self.labels.is_empty() {
                        // Wrap to last label
                        self.sidebar_selection = SidebarSelection::Label(self.labels.len() - 1);
                    } else {
                        // Wrap to Tomorrow
                        self.sidebar_selection = SidebarSelection::Tomorrow;
                    }
                }
            }
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
                (true, false) => std::cmp::Ordering::Less, // a (favorite) comes before b (non-favorite)
                (false, true) => std::cmp::Ordering::Greater, // a (non-favorite) comes after b (favorite)
                _ => a_project.name.cmp(&b_project.name),  // Same favorite status, sort by name
            }
        });
        sorted_projects
    }

    pub async fn load_local_data(&mut self, sync_service: &SyncService) {
        self.add_debug_log("📱 Loading local data into app...".to_string());
        self.loading = true;
        self.error_message = None;
        self.info_message = None;

        // Remember the current selection
        let current_selection = self.sidebar_selection.clone();

        // Load labels from local storage (fast)
        match sync_service.get_labels().await {
            Ok(labels) => {
                self.add_debug_log(format!("📱 Loaded {} labels into app", labels.len()));
                self.labels = labels;
            }
            Err(e) => {
                self.add_debug_log(format!("❌ Error loading labels: {e}"));
                self.error_message = Some(format!("Error loading labels: {e}"));
            }
        }

        // Load sections from local storage (fast)
        match sync_service.get_sections().await {
            Ok(sections) => {
                self.add_debug_log(format!("📱 Loaded {} sections into app", sections.len()));
                self.sections = sections;
            }
            Err(e) => {
                self.add_debug_log(format!("❌ Error loading sections: {e}"));
                self.error_message = Some(format!("Error loading sections: {e}"));
            }
        }

        // Load projects from local storage (fast)
        match sync_service.get_projects().await {
            Ok(projects) => {
                self.add_debug_log(format!("📱 Loaded {} projects into app", projects.len()));
                self.projects = projects;

                // Try to restore the previous selection or set a sensible default
                let mut selection_restored = false;

                match current_selection {
                    SidebarSelection::Today => {
                        self.sidebar_selection = SidebarSelection::Today;
                        selection_restored = true;
                    }
                    SidebarSelection::Tomorrow => {
                        self.sidebar_selection = SidebarSelection::Tomorrow;
                        selection_restored = true;
                    }
                    SidebarSelection::Label(index) => {
                        if index < self.labels.len() {
                            self.sidebar_selection = SidebarSelection::Label(index);
                            selection_restored = true;
                        }
                    }
                    SidebarSelection::Project(index) => {
                        if index < self.projects.len() {
                            self.sidebar_selection = SidebarSelection::Project(index);
                            selection_restored = true;
                        }
                    }
                }

                // If we couldn't restore the selection, set a sensible default (Today first)
                if !selection_restored {
                    self.sidebar_selection = SidebarSelection::Today;
                }

                // Update the list state to match the selection

                // Load tasks for the selected item
                if let Err(e) = self.load_tasks_for_selected_item(sync_service).await {
                    self.add_debug_log(format!("❌ Error loading tasks: {e}"));
                    self.error_message = Some(format!("Error loading tasks: {e}"));
                } else {
                    self.add_debug_log("📱 Tasks loaded successfully for selected item".to_string());
                }
            }
            Err(e) => {
                self.add_debug_log(format!("❌ Error loading projects: {e}"));
                self.error_message = Some(format!("Error loading projects: {e}"));
            }
        }

        self.loading = false;
    }

    /// Load tasks for the currently selected sidebar item (label or project)
    pub async fn load_tasks_for_selected_item(&mut self, sync_service: &SyncService) -> Result<(), anyhow::Error> {
        match &self.sidebar_selection {
            SidebarSelection::Today => {
                match sync_service.get_tasks_for_today().await {
                    Ok(tasks) => {
                        // Don't sort today's tasks since they're already sorted by the query (overdue first, then today)
                        self.tasks = tasks;
                        self.selected_task_index = 0;
                        // Update list state to highlight the first task
                        self.task_list_state.select(Some(0));
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Error loading today's tasks: {e}"));
                        return Err(e);
                    }
                }
            }
            SidebarSelection::Tomorrow => {
                match sync_service.get_tasks_for_tomorrow().await {
                    Ok(tasks) => {
                        // Don't sort tomorrow's tasks since they're already sorted by the query
                        self.tasks = tasks;
                        self.selected_task_index = 0;
                        // Update list state to highlight the first task
                        self.task_list_state.select(Some(0));
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Error loading tomorrow's tasks: {e}"));
                        return Err(e);
                    }
                }
            }
            SidebarSelection::Label(index) => {
                if let Some(label) = self.labels.get(*index) {
                    match sync_service.get_tasks_with_label(&label.name).await {
                        Ok(tasks) => {
                            self.tasks = self.sort_tasks(tasks);
                            self.selected_task_index = 0;
                            // Update list state to highlight the correct position (accounting for section headers)
                            if let Some(list_position) = self.get_task_list_position() {
                                self.task_list_state.select(Some(list_position));
                            }
                        }
                        Err(e) => {
                            self.error_message = Some(format!("Error loading tasks for label: {e}"));
                            return Err(e);
                        }
                    }
                }
            }
            SidebarSelection::Project(index) => {
                if let Some(project) = self.projects.get(*index) {
                    match sync_service.get_tasks_for_project(&project.id).await {
                        Ok(tasks) => {
                            self.tasks = self.sort_tasks(tasks);
                            self.selected_task_index = 0;
                            // Update list state to highlight the correct position (accounting for section headers)
                            if let Some(list_position) = self.get_task_list_position() {
                                self.task_list_state.select(Some(list_position));
                            }
                        }
                        Err(e) => {
                            self.error_message = Some(format!("Error loading tasks for project: {e}"));
                            return Err(e);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Sort tasks: pending first, then completed, then deleted
    fn sort_tasks(&self, mut tasks: Vec<TaskDisplay>) -> Vec<TaskDisplay> {
        tasks.sort_by(|a, b| {
            // Create priority scores: pending=0, completed=1, deleted=2
            let a_score = if a.is_deleted { 2 } else { i32::from(a.is_completed) };
            let b_score = if b.is_deleted { 2 } else { i32::from(b.is_completed) };

            // Sort by score (lower score = higher priority)
            a_score.cmp(&b_score)
        });
        tasks
    }

    pub fn next_task(&mut self) {
        if !self.tasks.is_empty() {
            self.selected_task_index = (self.selected_task_index + 1) % self.tasks.len();
            // Update list state to highlight the correct position (accounting for section headers)
            if let Some(list_position) = self.get_task_list_position() {
                self.task_list_state.select(Some(list_position));
            }
        }
    }

    pub fn previous_task(&mut self) {
        if !self.tasks.is_empty() {
            self.selected_task_index = if self.selected_task_index == 0 {
                self.tasks.len() - 1
            } else {
                self.selected_task_index - 1
            };
            // Update list state to highlight the correct position (accounting for section headers)
            if let Some(list_position) = self.get_task_list_position() {
                self.task_list_state.select(Some(list_position));
            }
        }
    }

    pub async fn toggle_selected_task(&mut self, sync_service: &SyncService) {
        if let Some(task) = self.tasks.get(self.selected_task_index) {
            self.completing_task = true;
            self.error_message = None;
            self.info_message = None;

            let result = if task.is_completed {
                sync_service.reopen_task(&task.id).await
            } else {
                sync_service.complete_task(&task.id).await
            };

            match result {
                Ok(()) => {
                    // Reload tasks to reflect the change
                    if let Err(e) = self.load_tasks_for_selected_item(sync_service).await {
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
            self.info_message = None;

            match sync_service.delete_task(&task.id).await {
                Ok(()) => {
                    // Reload tasks to reflect the change
                    if let Err(e) = self.load_tasks_for_selected_item(sync_service).await {
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

    /// Set the selected task's due date to today
    pub async fn set_selected_task_due_today(&mut self, sync_service: &SyncService) {
        if let Some(task) = self.tasks.get(self.selected_task_index) {
            if task.is_deleted {
                return; // Don't modify deleted tasks
            }

            self.error_message = None;
            self.info_message = None;

            let today = chrono::Utc::now()
                .date_naive()
                .format("%Y-%m-%d")
                .to_string();

            match sync_service
                .update_task_due_date(&task.id, Some(&today))
                .await
            {
                Ok(()) => {
                    // Try to sync first, but if it fails, at least reload local data
                    match sync_service.force_sync().await {
                        Ok(_) => {
                            // Sync succeeded, reload local data
                            if let Err(e) = self.load_tasks_for_selected_item(sync_service).await {
                                self.error_message = Some(format!("Error reloading tasks: {e}"));
                            } else {
                                self.info_message = Some("Task due date set to today!".to_string());
                            }
                        }
                        Err(e) => {
                            // Sync failed, but try to reload local data anyway
                            self.add_debug_log(format!("Warning: Sync failed after updating task due date: {e}"));
                            if let Err(e) = self.load_tasks_for_selected_item(sync_service).await {
                                self.error_message = Some(format!("Error reloading tasks: {e}"));
                            } else {
                                self.error_message =
                                    Some("Task due date updated but sync failed - data may be stale".to_string());
                            }
                        }
                    }
                }
                Err(e) => {
                    self.error_message = Some(format!("Error updating task due date: {e}"));
                }
            }
        }
    }

    /// Set the selected task's due date to tomorrow
    pub async fn set_selected_task_due_tomorrow(&mut self, sync_service: &SyncService) {
        if let Some(task) = self.tasks.get(self.selected_task_index) {
            if task.is_deleted {
                return; // Don't modify deleted tasks
            }

            self.error_message = None;
            self.info_message = None;

            let tomorrow = (chrono::Utc::now().date_naive() + chrono::Duration::days(1))
                .format("%Y-%m-%d")
                .to_string();

            match sync_service
                .update_task_due_date(&task.id, Some(&tomorrow))
                .await
            {
                Ok(()) => {
                    // Try to sync first, but if it fails, at least reload local data
                    match sync_service.force_sync().await {
                        Ok(_) => {
                            // Sync succeeded, reload local data
                            if let Err(e) = self.load_tasks_for_selected_item(sync_service).await {
                                self.error_message = Some(format!("Error reloading tasks: {e}"));
                            } else {
                                self.info_message = Some("Task due date set to tomorrow!".to_string());
                            }
                        }
                        Err(e) => {
                            // Sync failed, but try to reload local data anyway
                            self.add_debug_log(format!("Warning: Sync failed after updating task due date: {e}"));
                            if let Err(e) = self.load_tasks_for_selected_item(sync_service).await {
                                self.error_message = Some(format!("Error reloading tasks: {e}"));
                            } else {
                                self.error_message =
                                    Some("Task due date updated but sync failed - data may be stale".to_string());
                            }
                        }
                    }
                }
                Err(e) => {
                    self.error_message = Some(format!("Error updating task due date: {e}"));
                }
            }
        }
    }

    pub async fn force_clear_and_sync(&mut self, sync_service: &SyncService) {
        self.syncing = true;
        self.error_message = None;
        self.info_message = None;

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

    /// Start creating a new project
    pub fn start_create_project(&mut self) {
        self.creating_project = true;
        self.new_project_name.clear();
        self.new_project_parent_id = None;
    }

    /// Cancel project creation
    pub fn cancel_create_project(&mut self) {
        self.creating_project = false;
        self.new_project_name.clear();
        self.new_project_parent_id = None;
    }

    /// Add a character to the project name
    pub fn add_char_to_project_name(&mut self, c: char) {
        if self.creating_project {
            self.new_project_name.push(c);
        } else if self.editing_project {
            self.edit_project_name.push(c);
        }
    }

    /// Remove the last character from the project name
    pub fn remove_char_from_project_name(&mut self) {
        if self.creating_project && !self.new_project_name.is_empty() {
            self.new_project_name.pop();
        } else if self.editing_project && !self.edit_project_name.is_empty() {
            self.edit_project_name.pop();
        }
    }

    /// Cycle through parent project options
    pub fn cycle_parent_project(&mut self) {
        if !self.creating_project || self.projects.is_empty() {
            return;
        }

        if let Some(current_parent) = &self.new_project_parent_id {
            // Find current parent index and move to next
            if let Some(current_idx) = self.projects.iter().position(|p| p.id == *current_parent) {
                let next_idx = (current_idx + 1) % self.projects.len();
                self.new_project_parent_id = Some(self.projects[next_idx].id.clone());
            } else {
                // Current parent not found, start with first project
                self.new_project_parent_id = Some(self.projects[0].id.clone());
            }
        } else {
            // No parent selected, start with first project
            self.new_project_parent_id = Some(self.projects[0].id.clone());
        }
    }

    /// Create the new project
    pub async fn create_project(&mut self, sync_service: &SyncService) {
        if self.new_project_name.trim().is_empty() {
            self.error_message = Some("Project name cannot be empty".to_string());
            return;
        }

        self.creating_project = false;
        self.error_message = None;
        self.info_message = None;

        match sync_service
            .create_project(self.new_project_name.trim(), self.new_project_parent_id.as_deref())
            .await
        {
            Ok(()) => {
                // Try to sync first, but if it fails, at least reload local data
                match sync_service.force_sync().await {
                    Ok(_) => {
                        // Sync succeeded, reload local data
                        self.load_local_data(sync_service).await;
                        self.error_message = Some("Project created successfully!".to_string());
                    }
                    Err(e) => {
                        // Sync failed, but try to reload local data anyway
                        self.add_debug_log(format!("Warning: Sync failed after project creation: {e}"));
                        self.load_local_data(sync_service).await;
                        self.error_message = Some("Project created but sync failed - data may be stale".to_string());
                    }
                }

                // Clear the success message after a short delay
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                self.error_message = None;
                self.info_message = None;
            }
            Err(e) => {
                self.error_message = Some(format!("Error creating project: {e}"));
            }
        }

        self.new_project_name.clear();
        self.new_project_parent_id = None;
    }

    /// Start project deletion confirmation
    pub fn start_delete_project(&mut self) {
        if let Some(project) = self.get_selected_project() {
            self.delete_project_confirmation = Some(project.id.clone());
        }
    }

    /// Cancel project deletion
    pub fn cancel_delete_project(&mut self) {
        self.delete_project_confirmation = None;
    }

    /// Delete the confirmed project
    pub async fn delete_project(&mut self, sync_service: &SyncService) {
        if let Some(project_id) = &self.delete_project_confirmation {
            self.error_message = None;
            self.info_message = None;

            match sync_service.delete_project(project_id).await {
                Ok(()) => {
                    // Force a full sync to ensure all data is up to date
                    self.force_clear_and_sync(sync_service).await;
                    self.info_message = Some("Project deleted successfully!".to_string());
                }
                Err(e) => {
                    self.error_message = Some(format!("Error deleting project: {e}"));
                }
            }

            self.delete_project_confirmation = None;
        }
    }

    /// Start creating a new task
    pub fn start_create_task(&mut self) {
        self.creating_task = true;
        self.new_task_content.clear();
        // Set the current project as the default project for the new task (only if a project is selected)
        if let Some(project) = self.get_selected_project() {
            self.new_task_project_id = Some(project.id.clone());
        }
    }

    /// Cancel task creation
    pub fn cancel_create_task(&mut self) {
        self.creating_task = false;
        self.new_task_content.clear();
        self.new_task_project_id = None;
    }

    /// Start editing the currently selected project
    pub fn start_edit_project(&mut self) {
        if let Some(project) = self.get_selected_project() {
            let project_name = project.name.clone();
            let project_id = project.id.clone();
            self.editing_project = true;
            self.edit_project_name = project_name;
            self.edit_project_id = Some(project_id);
        }
    }

    /// Cancel project editing
    pub fn cancel_edit_project(&mut self) {
        self.editing_project = false;
        self.edit_project_name.clear();
        self.edit_project_id = None;
    }

    /// Start editing the currently selected task
    pub fn start_edit_task(&mut self) {
        if let Some(task) = self.tasks.get(self.selected_task_index) {
            if !task.is_deleted {
                self.editing_task = true;
                self.edit_task_content = task.content.clone();
                self.edit_task_id = Some(task.id.clone());
            }
        }
    }

    /// Cancel task editing
    pub fn cancel_edit_task(&mut self) {
        self.editing_task = false;
        self.edit_task_content.clear();
        self.edit_task_id = None;
    }

    /// Add a character to the task content
    pub fn add_char_to_task_content(&mut self, c: char) {
        if self.creating_task {
            self.new_task_content.push(c);
        } else if self.editing_task {
            self.edit_task_content.push(c);
        }
    }

    /// Remove the last character from the task content
    pub fn remove_char_from_task_content(&mut self) {
        if self.creating_task && !self.new_task_content.is_empty() {
            self.new_task_content.pop();
        } else if self.editing_task && !self.edit_task_content.is_empty() {
            self.edit_task_content.pop();
        }
    }

    /// Create the new task
    pub async fn create_task(&mut self, sync_service: &SyncService) {
        if self.new_task_content.trim().is_empty() {
            self.error_message = Some("Task content cannot be empty".to_string());
            return;
        }

        self.creating_task = false;
        self.error_message = None;
        self.info_message = None;

        // Create the task in the currently selected project (if a project is selected)
        if let Some(project) = self.get_selected_project() {
            match sync_service
                .create_task(self.new_task_content.trim(), Some(&project.id))
                .await
            {
                Ok(()) => {
                    // Try to sync first, but if it fails, at least reload local data
                    match sync_service.force_sync().await {
                        Ok(_) => {
                            // Sync succeeded, reload local data
                            self.load_local_data(sync_service).await;
                            self.info_message = Some("Task created successfully!".to_string());
                        }
                        Err(e) => {
                            // Sync failed, but try to reload local data anyway
                            self.add_debug_log(format!("Warning: Sync failed after task creation: {e}"));
                            self.load_local_data(sync_service).await;
                            self.error_message = Some("Task created but sync failed - data may be stale".to_string());
                        }
                    }

                    // Clear the success message after a short delay
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    self.error_message = None;
                    self.info_message = None;
                }
                Err(e) => {
                    self.error_message = Some(format!("Error creating task: {e}"));
                }
            }
        } else {
            self.error_message =
                Some("Please select a project to create a task (labels cannot contain tasks directly)".to_string());
        }

        self.new_task_content.clear();
        self.new_task_project_id = None;
    }

    /// Update the task with edited content
    pub async fn save_edit_task(&mut self, sync_service: &SyncService) {
        if self.edit_task_content.trim().is_empty() {
            self.error_message = Some("Task content cannot be empty".to_string());
            return;
        }

        if let Some(task_id) = &self.edit_task_id.clone() {
            self.editing_task = false;
            self.error_message = None;
            self.info_message = None;

            match sync_service
                .update_task_content(task_id, self.edit_task_content.trim())
                .await
            {
                Ok(()) => {
                    // Try to sync first, but if it fails, at least reload local data
                    match sync_service.force_sync().await {
                        Ok(_) => {
                            // Sync succeeded, reload local data
                            self.load_local_data(sync_service).await;
                            self.info_message = Some("Task updated successfully!".to_string());
                        }
                        Err(e) => {
                            // Sync failed but task was updated, just reload local data
                            self.load_local_data(sync_service).await;
                            self.error_message = Some(format!("Task updated but sync failed: {e}"));
                        }
                    }
                }
                Err(e) => {
                    self.error_message = Some(format!("Failed to update task: {e}"));
                }
            }
        }

        self.edit_task_id = None;
        self.edit_task_content.clear();
    }

    /// Update the project with edited content
    pub async fn save_edit_project(&mut self, sync_service: &SyncService) {
        if self.edit_project_name.trim().is_empty() {
            self.error_message = Some("Project name cannot be empty".to_string());
            return;
        }

        if let Some(project_id) = &self.edit_project_id.clone() {
            self.editing_project = false;
            self.error_message = None;
            self.info_message = None;

            match sync_service
                .update_project_content(project_id, self.edit_project_name.trim())
                .await
            {
                Ok(()) => {
                    // Try to sync first, but if it fails, at least reload local data
                    match sync_service.force_sync().await {
                        Ok(_) => {
                            // Sync succeeded, reload local data
                            self.load_local_data(sync_service).await;
                            self.info_message = Some("Project updated successfully!".to_string());
                        }
                        Err(e) => {
                            // Sync failed but project was updated, just reload local data
                            self.load_local_data(sync_service).await;
                            self.error_message = Some(format!("Project updated but sync failed: {e}"));
                        }
                    }
                }
                Err(e) => {
                    self.error_message = Some(format!("Failed to update project: {e}"));
                }
            }
        }

        self.edit_project_id = None;
        self.edit_project_name.clear();
    }

    /// Start creating a new label
    pub fn start_create_label(&mut self) {
        self.creating_label = true;
        self.new_label_name.clear();
    }

    /// Cancel label creation
    pub fn cancel_create_label(&mut self) {
        self.creating_label = false;
        self.new_label_name.clear();
    }

    /// Start editing the currently selected label
    pub fn start_edit_label(&mut self) {
        if let Some(label) = self.get_selected_label() {
            let label_name = label.name.clone();
            let label_id = label.id.clone();
            self.editing_label = true;
            self.edit_label_name = label_name;
            self.edit_label_id = Some(label_id);
        }
    }

    /// Cancel label editing
    pub fn cancel_edit_label(&mut self) {
        self.editing_label = false;
        self.edit_label_name.clear();
        self.edit_label_id = None;
    }

    /// Add a character to the label name
    pub fn add_char_to_label_name(&mut self, c: char) {
        if self.creating_label {
            self.new_label_name.push(c);
        } else if self.editing_label {
            self.edit_label_name.push(c);
        }
    }

    /// Remove the last character from the label name
    pub fn remove_char_from_label_name(&mut self) {
        if self.creating_label && !self.new_label_name.is_empty() {
            self.new_label_name.pop();
        } else if self.editing_label && !self.edit_label_name.is_empty() {
            self.edit_label_name.pop();
        }
    }

    /// Create the new label
    pub async fn create_label(&mut self, sync_service: &SyncService) {
        if self.new_label_name.trim().is_empty() {
            self.error_message = Some("Label name cannot be empty".to_string());
            return;
        }

        self.creating_label = false;
        self.error_message = None;
        self.info_message = None;

        match sync_service
            .create_label(self.new_label_name.trim(), None)
            .await
        {
            Ok(()) => {
                // Try to sync first, but if it fails, at least reload local data
                match sync_service.force_sync().await {
                    Ok(_) => {
                        // Sync succeeded, reload local data
                        self.load_local_data(sync_service).await;
                        self.info_message = Some("Label created successfully!".to_string());
                    }
                    Err(e) => {
                        // Sync failed, but try to reload local data anyway
                        self.add_debug_log(format!("Warning: Sync failed after label creation: {e}"));
                        self.load_local_data(sync_service).await;
                        self.error_message = Some("Label created but sync failed - data may be stale".to_string());
                    }
                }
            }
            Err(e) => {
                self.error_message = Some(format!("Error creating label: {e}"));
            }
        }

        self.new_label_name.clear();
    }

    /// Update the label with edited content
    pub async fn save_edit_label(&mut self, sync_service: &SyncService) {
        if self.edit_label_name.trim().is_empty() {
            self.error_message = Some("Label name cannot be empty".to_string());
            return;
        }

        if let Some(label_id) = &self.edit_label_id.clone() {
            self.editing_label = false;
            self.error_message = None;
            self.info_message = None;

            match sync_service
                .update_label_content(label_id, self.edit_label_name.trim())
                .await
            {
                Ok(()) => {
                    // Try to sync first, but if it fails, at least reload local data
                    match sync_service.force_sync().await {
                        Ok(_) => {
                            // Sync succeeded, reload local data
                            self.load_local_data(sync_service).await;
                            self.info_message = Some("Label updated successfully!".to_string());
                        }
                        Err(e) => {
                            // Sync failed but label was updated, just reload local data
                            self.load_local_data(sync_service).await;
                            self.error_message = Some(format!("Label updated but sync failed: {e}"));
                        }
                    }
                }
                Err(e) => {
                    self.error_message = Some(format!("Failed to update label: {e}"));
                }
            }
        }

        self.edit_label_id = None;
        self.edit_label_name.clear();
    }

    /// Start label deletion confirmation
    pub fn start_delete_label(&mut self) {
        if let Some(label) = self.get_selected_label() {
            self.delete_label_confirmation = Some(label.id.clone());
        }
    }

    /// Cancel label deletion
    pub fn cancel_delete_label(&mut self) {
        self.delete_label_confirmation = None;
    }

    /// Delete the confirmed label
    pub async fn delete_label(&mut self, sync_service: &SyncService) {
        if let Some(label_id) = &self.delete_label_confirmation {
            self.error_message = None;
            self.info_message = None;

            match sync_service.delete_label(label_id).await {
                Ok(()) => {
                    // Force a full sync to ensure all data is up to date
                    self.force_clear_and_sync(sync_service).await;
                    self.info_message = Some("Label deleted successfully!".to_string());
                }
                Err(e) => {
                    self.error_message = Some(format!("Error deleting label: {e}"));
                }
            }

            self.delete_label_confirmation = None;
        }
    }
}
