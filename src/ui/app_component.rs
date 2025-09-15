use crate::logger::Logger;
use crate::sync::{SyncService, SyncStatus};
use crate::todoist::{LabelDisplay, ProjectDisplay, SectionDisplay, TaskDisplay};
use crate::ui::components::{DialogComponent, SidebarComponent, TaskListComponent};
use crate::ui::core::SidebarSelection;
use crate::ui::core::{
    actions::{Action, DialogType},
    event_handler::EventType,
    task_manager::{TaskId, TaskManager},
    Component,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    Frame,
};
use tokio::sync::mpsc;

/// Application state separate from UI concerns
#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub projects: Vec<ProjectDisplay>,
    pub tasks: Vec<TaskDisplay>,
    pub labels: Vec<LabelDisplay>,
    pub sections: Vec<SectionDisplay>,
    pub sidebar_selection: SidebarSelection,
    pub loading: bool,
    pub error_message: Option<String>,
    pub info_message: Option<String>,
    pub show_help: bool,
    /// didnt we just got rid of custom scrolling ?
    pub help_scroll_offset: usize,
}

impl AppState {
    /// Update all data at once
    pub fn update_data(
        &mut self,
        projects: Vec<ProjectDisplay>,
        labels: Vec<LabelDisplay>,
        sections: Vec<SectionDisplay>,
        tasks: Vec<TaskDisplay>,
    ) {
        self.projects = projects;
        self.labels = labels;
        self.sections = sections;
        self.tasks = tasks;
    }

    /// Clear any transient messages
    pub fn clear_messages(&mut self) {
        self.error_message = None;
        self.info_message = None;
    }
}

pub struct AppComponent {
    // Component composition
    sidebar: SidebarComponent,
    task_list: TaskListComponent,
    dialog: DialogComponent,

    // Application state
    state: AppState,

    // Services
    sync_service: SyncService,
    task_manager: TaskManager,
    background_action_rx: mpsc::UnboundedReceiver<Action>,
    logger: Logger,

    // Simple UI state
    should_quit: bool,
    active_sync_task: Option<TaskId>,
}

impl AppComponent {
    pub fn new(mut sync_service: SyncService) -> Self {
        let sidebar = SidebarComponent::new();
        let task_list = TaskListComponent::new();
        let (task_manager, background_action_rx) = TaskManager::new();
        let logger = Logger::new();
        sync_service.set_logger(logger.clone());

        let state = AppState {
            loading: true,
            ..Default::default()
        };

        Self {
            sidebar,
            task_list,
            dialog: DialogComponent::new(),
            state,
            sync_service,
            task_manager,
            background_action_rx,
            logger,
            should_quit: false,
            active_sync_task: None,
        }
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// Get the number of active background tasks
    pub fn active_task_count(&self) -> usize {
        self.task_manager.task_count()
    }

    /// Check if currently syncing
    pub fn is_syncing(&self) -> bool {
        self.active_sync_task.is_some()
    }

    /// Get total number of tasks
    pub fn total_tasks(&self) -> usize {
        self.state.tasks.len()
    }

    /// Get total number of projects
    pub fn total_projects(&self) -> usize {
        self.state.projects.len()
    }

    /// Trigger initial sync on startup
    pub fn trigger_initial_sync(&mut self) {
        self.logger.log("AppComponent: Starting initial sync".to_string());
        if self.active_sync_task.is_none() {
            self.start_background_sync();
            // Data fetch will be triggered automatically when sync completes
            self.logger.log("AppComponent: Initial sync scheduled".to_string());
        }
    }

    /// Update all components with current data
    fn sync_component_data(&mut self) {
        // Update sidebar
        self.sidebar.update_data(self.state.projects.clone(), self.state.labels.clone());
        self.sidebar.selection = self.state.sidebar_selection.clone();

        // Update task list
        self.task_list.update_data(
            self.state.tasks.clone(),
            self.state.sections.clone(),
            self.state.projects.clone(),
            self.state.labels.clone(),
            self.state.sidebar_selection.clone(),
        );

        // Update dialog
        self.dialog.update_data(self.state.projects.clone(), self.state.labels.clone());
        self.dialog.set_logger(self.logger.clone());
    }

    /// Handle global keyboard shortcuts that aren't component-specific
    fn handle_global_key(&mut self, key: KeyEvent) -> Action {
        // Handle help panel scrolling when help is open
        if self.state.show_help {
            match key.code {
                KeyCode::Up => return Action::HelpScrollUp,
                KeyCode::Down => return Action::HelpScrollDown,
                KeyCode::Home => return Action::HelpScrollToTop,
                KeyCode::End => return Action::HelpScrollToBottom,
                KeyCode::Char('?') | KeyCode::Esc => return Action::ShowHelp(false),
                _ => {} // Continue to other key handling
            }
        }

        match key.code {
            KeyCode::Char('q') => {
                self.logger.log("Global key: 'q' - quitting application".to_string());
                Action::Quit
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.logger.log("Global key: Ctrl+C - quitting application".to_string());
                Action::Quit
            }
            KeyCode::Char('?') | KeyCode::Char('h') => {
                self.logger.log("Global key: '?' or 'h' - opening help dialog".to_string());
                Action::ShowDialog(DialogType::Help)
            }
            KeyCode::Char('G') => {
                self.logger.log("Global key: 'G' - opening logs dialog".to_string());
                Action::ShowDialog(DialogType::Logs)
            }
            KeyCode::Char('A') => {
                self.logger.log("Global key: 'A' - opening project creation dialog".to_string());
                Action::ShowDialog(DialogType::ProjectCreation)
            }
            KeyCode::Char('D') => {
                // Delete current project (only if a project is selected)
                match &self.state.sidebar_selection {
                    SidebarSelection::Project(index) => {
                        if let Some(project) = self.state.projects.get(*index) {
                            self.logger.log(format!(
                                "Global key: 'D' - deleting project '{}' (ID: {})",
                                project.name, project.id
                            ));
                            Action::ShowDialog(DialogType::DeleteConfirmation {
                                item_type: "project".to_string(),
                                item_id: project.id.clone(),
                            })
                        } else {
                            self.logger
                                .log("Global key: 'D' - no project selected (invalid index)".to_string());
                            Action::ShowDialog(DialogType::Error("No project selected to delete".to_string()))
                        }
                    }
                    SidebarSelection::Today => {
                        self.logger.log("Global key: 'D' - cannot delete Today view".to_string());
                        Action::ShowDialog(DialogType::Info("Cannot delete the Today view".to_string()))
                    }
                    SidebarSelection::Tomorrow => {
                        self.logger.log("Global key: 'D' - cannot delete Tomorrow view".to_string());
                        Action::ShowDialog(DialogType::Info("Cannot delete the Tomorrow view".to_string()))
                    }
                    SidebarSelection::Upcoming => {
                        self.logger.log("Global key: 'D' - cannot delete Upcoming view".to_string());
                        Action::ShowDialog(DialogType::Info("Cannot delete the Upcoming view".to_string()))
                    }
                    SidebarSelection::Label(index) => {
                        if let Some(label) = self.state.labels.get(*index) {
                            self.logger.log(format!(
                                "Global key: 'D' - deleting label '{}' (ID: {})",
                                label.name, label.id
                            ));
                            Action::ShowDialog(DialogType::DeleteConfirmation {
                                item_type: "label".to_string(),
                                item_id: label.id.clone(),
                            })
                        } else {
                            self.logger
                                .log("Global key: 'D' - no label selected (invalid index)".to_string());
                            Action::ShowDialog(DialogType::Error("No label selected to delete".to_string()))
                        }
                    }
                }
            }
            KeyCode::Char('E') => {
                // Edit current sidebar selection (project or label)
                match &self.state.sidebar_selection {
                    SidebarSelection::Project(index) => {
                        if let Some(project) = self.state.projects.get(*index) {
                            self.logger.log(format!(
                                "Global key: 'E' - editing project '{}' (ID: {})",
                                project.name, project.id
                            ));
                            Action::ShowDialog(DialogType::ProjectEdit {
                                project_id: project.id.clone(),
                                name: project.name.clone(),
                            })
                        } else {
                            self.logger
                                .log("Global key: 'E' - no project selected (invalid index)".to_string());
                            Action::ShowDialog(DialogType::Error("No project selected to edit".to_string()))
                        }
                    }
                    SidebarSelection::Today => {
                        self.logger.log("Global key: 'E' - cannot edit Today view".to_string());
                        Action::ShowDialog(DialogType::Info("Cannot edit the Today view".to_string()))
                    }
                    SidebarSelection::Tomorrow => {
                        self.logger.log("Global key: 'E' - cannot edit Tomorrow view".to_string());
                        Action::ShowDialog(DialogType::Info("Cannot edit the Tomorrow view".to_string()))
                    }
                    SidebarSelection::Upcoming => {
                        self.logger.log("Global key: 'E' - cannot edit Upcoming view".to_string());
                        Action::ShowDialog(DialogType::Info("Cannot edit the Upcoming view".to_string()))
                    }
                    SidebarSelection::Label(index) => {
                        if let Some(label) = self.state.labels.get(*index) {
                            self.logger.log(format!(
                                "Global key: 'E' - editing label '{}' (ID: {})",
                                label.name, label.id
                            ));
                            Action::ShowDialog(DialogType::LabelEdit {
                                label_id: label.id.clone(),
                                name: label.name.clone(),
                            })
                        } else {
                            self.logger
                                .log("Global key: 'E' - no label selected (invalid index)".to_string());
                            Action::ShowDialog(DialogType::Error("No label selected to edit".to_string()))
                        }
                    }
                }
            }
            KeyCode::Char('r') => {
                self.logger.log("Global key: 'r' - starting manual sync".to_string());
                Action::StartSync
            }
            KeyCode::Esc => {
                if self.dialog.is_visible() {
                    self.logger.log("Global key: Esc - closing dialog".to_string());
                    Action::HideDialog
                } else {
                    self.logger.log("Global key: Esc - quitting application".to_string());
                    Action::Quit
                }
            }
            _ => Action::None,
        }
    }

    /// Handle app-level actions that require business logic
    pub async fn handle_app_action(&mut self, action: Action) -> Action {
        match action {
            Action::Quit => {
                self.should_quit = true;
                Action::None
            }
            Action::StartSync => {
                if self.active_sync_task.is_none() {
                    self.logger.log("Starting background sync".to_string());
                    self.state.loading = true;
                    self.start_background_sync();
                } else {
                    self.logger.log("Sync already in progress, ignoring".to_string());
                }
                Action::None
            }
            Action::SyncCompleted(status) => {
                self.logger.log(format!("Sync: Completed with status {:?}", status));
                self.active_sync_task = None;
                self.state.loading = false;

                // Extract data from sync status and update components
                self.update_data_from_sync(status);
                self.sync_component_data();

                self.state.info_message = Some("Sync completed successfully".to_string());
                self.logger.log("Sync: Showing completion info dialog".to_string());
                Action::ShowDialog(DialogType::Info(self.state.info_message.clone().unwrap()))
            }
            Action::SyncFailed(error) => {
                self.logger.log(format!("Sync: Failed with error: {}", error));
                self.active_sync_task = None;
                self.state.loading = false;
                self.state.error_message = Some(error);
                Action::ShowDialog(DialogType::Error(self.state.error_message.clone().unwrap_or_default()))
            }
            Action::ShowDialog(ref dialog_type) => {
                self.logger.log(format!("Dialog: Showing dialog {:?}", dialog_type));
                // Dialog component will handle the actual dialog setup
                action
            }
            Action::HideDialog => {
                self.logger.log("Dialog: Hiding current dialog".to_string());
                // Dialog component will handle hiding
                action
            }
            Action::NavigateToSidebar(selection) => {
                // Create a more detailed log message with names
                let selection_desc = match &selection {
                    SidebarSelection::Today => "Today".to_string(),
                    SidebarSelection::Tomorrow => "Tomorrow".to_string(),
                    SidebarSelection::Upcoming => "Upcoming".to_string(),
                    SidebarSelection::Project(index) => {
                        if let Some(project) = self.state.projects.get(*index) {
                            format!("Project({}) '{}'", index, project.name)
                        } else {
                            format!("Project({}) [unknown]", index)
                        }
                    }
                    SidebarSelection::Label(index) => {
                        if let Some(label) = self.state.labels.get(*index) {
                            format!("Label({}) '{}'", index, label.name)
                        } else {
                            format!("Label({}) [unknown]", index)
                        }
                    }
                };

                self.logger
                    .log(format!("Navigation: Sidebar selection changed to {}", selection_desc));
                self.state.sidebar_selection = selection.clone();
                // Reload data for the new selection
                self.schedule_data_fetch();
                self.logger
                    .log("Navigation: Scheduled data fetch for new selection".to_string());
                Action::None
            }
            // Task operations with background execution
            Action::CreateTask { content, project_id } => {
                let project_desc = match &project_id {
                    Some(id) => format!(" in project {}", id),
                    None => " in inbox".to_string(),
                };
                self.logger.log(format!(
                    "Task: Creating task with content '{}'{}",
                    content, project_desc
                ));

                // Format task info to include both content and project_id
                let task_info = match project_id {
                    Some(pid) => format!("{}|{}", content, pid),
                    None => content,
                };
                self.spawn_task_operation("Create task".to_string(), task_info);
                Action::None
            }
            Action::CompleteTask(task_id) => {
                // Find the task being completed
                let sync_service = self.sync_service.clone();
                if let Ok(Some(task)) = sync_service.get_task_by_id(&task_id).await {
                    let task_desc = format!("ID {} '{}'", task_id, task.content);

                    self.logger.log(format!("Task: Completing task {}", task_desc));

                    // Todoist API automatically handles subtasks when parent is completed
                    self.spawn_task_operation("Complete task".to_string(), task_id);
                } else {
                    self.logger.log(format!("Task: Cannot complete - task {} not found", task_id));
                }
                Action::None
            }
            Action::CyclePriority(task_id) => {
                // Find task and cycle its priority
                let sync_service = self.sync_service.clone();
                if let Ok(Some(task)) = sync_service.get_task_by_id(&task_id).await {
                    // Todoist priorities: 1 (Normal), 2 (High), 3 (Higher), 4 (Highest)
                    let new_priority = match task.priority {
                        4 => 1,                 // Highest -> Normal
                        _ => task.priority + 1, // Normal/High/Higher -> next level
                    };
                    let task_desc = format!(
                        "ID {} '{}' (P{} -> P{})",
                        task_id, task.content, task.priority, new_priority
                    );
                    self.logger.log(format!("Task: Cycling priority for task {}", task_desc));
                    self.spawn_task_operation("Cycle priority".to_string(), format!("{}|{}", task_id, new_priority));
                } else {
                    self.logger
                        .log(format!("Task: Cannot cycle priority - task {} not found", task_id));
                }
                Action::None
            }
            Action::DeleteTask(task_id) => {
                // Find task name for better logging
                let sync_service = self.sync_service.clone();
                let task_desc = if let Ok(Some(task)) = sync_service.get_task_by_id(&task_id).await {
                    format!("ID {} '{}'", task_id, task.content)
                } else {
                    format!("ID {} [unknown]", task_id)
                };
                self.logger.log(format!("Task: Deleting task {}", task_desc));
                self.spawn_task_operation("Delete task".to_string(), task_id);
                Action::None
            }
            Action::SetTaskDueToday(task_id) => {
                // Find task name for better logging
                let sync_service = self.sync_service.clone();
                let task_desc = if let Ok(Some(task)) = sync_service.get_task_by_id(&task_id).await {
                    format!("ID {} '{}'", task_id, task.content)
                } else {
                    format!("ID {} [unknown]", task_id)
                };
                self.logger
                    .log(format!("Task: Setting due date to today for task {}", task_desc));
                self.spawn_task_operation("Set task due today".to_string(), format!("{}|today", task_id));
                Action::None
            }
            Action::SetTaskDueTomorrow(task_id) => {
                // Find task name for better logging
                let sync_service = self.sync_service.clone();
                let task_desc = if let Ok(Some(task)) = sync_service.get_task_by_id(&task_id).await {
                    format!("ID {} '{}'", task_id, task.content)
                } else {
                    format!("ID {} [unknown]", task_id)
                };
                self.logger
                    .log(format!("Task: Setting due date to tomorrow for task {}", task_desc));
                self.spawn_task_operation("Set task due tomorrow".to_string(), format!("{}|tomorrow", task_id));
                Action::None
            }
            Action::SetTaskDueNextWeek(task_id) => {
                // Find task name for better logging
                let sync_service = self.sync_service.clone();
                let task_desc = if let Ok(Some(task)) = sync_service.get_task_by_id(&task_id).await {
                    format!("ID {} '{}'", task_id, task.content)
                } else {
                    format!("ID {} [unknown]", task_id)
                };
                self.logger
                    .log(format!("Task: Setting due date to next week for task {}", task_desc));
                self.spawn_task_operation("Set task due next week".to_string(), format!("{}|next_week", task_id));
                Action::None
            }
            Action::SetTaskDueWeekEnd(task_id) => {
                // Find task name for better logging
                let sync_service = self.sync_service.clone();
                let task_desc = if let Ok(Some(task)) = sync_service.get_task_by_id(&task_id).await {
                    format!("ID {} '{}'", task_id, task.content)
                } else {
                    format!("ID {} [unknown]", task_id)
                };
                self.logger
                    .log(format!("Task: Setting due date to weekend for task {}", task_desc));
                self.spawn_task_operation("Set task due weekend".to_string(), format!("{}|weekend", task_id));
                Action::None
            }
            Action::EditTask { id, content } => {
                self.logger
                    .log(format!("Task: Editing task ID {} with new content '{}'", id, content));
                self.spawn_task_operation("Edit task".to_string(), format!("{}: {}", id, content));
                Action::None
            }
            Action::CreateProject { name, parent_id } => {
                let parent_desc = match &parent_id {
                    Some(id) => format!(" with parent {}", id),
                    None => "".to_string(),
                };
                self.logger.log(format!("Project: Creating project '{}'{}", name, parent_desc));

                // Format project info to include both name and parent_id
                let project_info = match parent_id {
                    Some(pid) => format!("{}|{}", name, pid),
                    None => name,
                };
                self.spawn_task_operation("Create project".to_string(), project_info);
                Action::None
            }
            Action::DeleteProject(project_id) => {
                // Find project name for better logging
                let project_desc = if let Some(project) = self.state.projects.iter().find(|p| p.id == project_id) {
                    format!("ID {} '{}'", project_id, project.name)
                } else {
                    format!("ID {} [unknown]", project_id)
                };
                self.logger.log(format!("Project: Deleting project {}", project_desc));
                self.spawn_task_operation("Delete project".to_string(), project_id);
                Action::None
            }
            Action::DeleteLabel(label_id) => {
                // Find label name for better logging
                let label_desc = if let Some(label) = self.state.labels.iter().find(|l| l.id == label_id) {
                    format!("ID {} '{}'", label_id, label.name)
                } else {
                    format!("ID {} [unknown]", label_id)
                };
                self.logger.log(format!("Label: Deleting label {}", label_desc));
                self.spawn_task_operation("Delete label".to_string(), label_id);
                Action::None
            }
            Action::CreateLabel { name } => {
                self.logger.log(format!("Label: Creating label '{}'", name));
                self.spawn_task_operation("Create label".to_string(), name);
                Action::None
            }
            Action::EditProject { id, name } => {
                // Find project name for better logging
                let project_desc = if let Some(project) = self.state.projects.iter().find(|p| p.id == id) {
                    format!("ID {} '{}' -> '{}'", id, project.name, name)
                } else {
                    format!("ID {} [unknown] -> '{}'", id, name)
                };
                self.logger.log(format!("Project: Editing project {}", project_desc));
                self.spawn_task_operation("Edit project".to_string(), format!("{}: {}", id, name));
                Action::None
            }
            Action::EditLabel { id, name } => {
                // Find label name for better logging
                let label_desc = if let Some(label) = self.state.labels.iter().find(|l| l.id == id) {
                    format!("ID {} '{}' -> '{}'", id, label.name, name)
                } else {
                    format!("ID {} [unknown] -> '{}'", id, name)
                };
                self.logger.log(format!("Label: Editing label {}", label_desc));
                self.spawn_task_operation("Edit label".to_string(), format!("{}: {}", id, name));
                Action::None
            }
            Action::DataLoaded {
                projects,
                labels,
                sections,
                tasks,
            } => {
                // Create detailed log with current selection context
                let selection_context = match &self.state.sidebar_selection {
                    SidebarSelection::Today => "Today view".to_string(),
                    SidebarSelection::Tomorrow => "Tomorrow view".to_string(),
                    SidebarSelection::Upcoming => "Upcoming view".to_string(),
                    SidebarSelection::Project(index) => {
                        if let Some(project) = projects.get(*index) {
                            format!("Project '{}'", project.name)
                        } else {
                            format!("Project({})", index)
                        }
                    }
                    SidebarSelection::Label(index) => {
                        if let Some(label) = labels.get(*index) {
                            format!("Label '{}'", label.name)
                        } else {
                            format!("Label({})", index)
                        }
                    }
                };

                self.logger.log(format!(
                    "Data: Loaded {} projects, {} labels, {} sections, {} tasks for {}",
                    projects.len(),
                    labels.len(),
                    sections.len(),
                    tasks.len(),
                    selection_context
                ));
                // Update app state with loaded data
                self.state.update_data(projects, labels, sections, tasks);
                self.sync_component_data();
                self.logger.log("Data: Updated all component data after data load".to_string());
                Action::None
            }
            Action::NextTask => {
                self.logger.log("Navigation: Next task (j/down)".to_string());
                action
            }
            Action::PreviousTask => {
                self.logger.log("Navigation: Previous task (k/up)".to_string());
                action
            }
            Action::RefreshData => {
                self.logger.log("Data: Refreshing UI data after task operation".to_string());
                // Schedule a data fetch to reload current view with updated data
                self.schedule_data_fetch();
                Action::None
            }
            // Help panel scrolling actions
            Action::HelpScrollUp => {
                if self.state.help_scroll_offset > 0 {
                    self.state.help_scroll_offset -= 1;
                }
                self.logger.log(format!(
                    "Help: Scrolled up, offset now {}",
                    self.state.help_scroll_offset
                ));
                Action::None
            }
            Action::HelpScrollDown => {
                self.state.help_scroll_offset += 1;
                self.logger.log(format!(
                    "Help: Scrolled down, offset now {}",
                    self.state.help_scroll_offset
                ));
                Action::None
            }
            Action::HelpScrollToTop => {
                self.state.help_scroll_offset = 0;
                self.logger.log("Help: Scrolled to top".to_string());
                Action::None
            }
            Action::HelpScrollToBottom => {
                // Set to a large value - dialog component will handle bounds checking
                self.state.help_scroll_offset = usize::MAX;
                self.logger.log("Help: Scrolled to bottom".to_string());
                Action::None
            }
            Action::ShowHelp(show) => {
                self.state.show_help = show;
                if !show {
                    // Reset scroll when hiding help
                    self.state.help_scroll_offset = 0;
                }
                self.logger
                    .log(format!("Help: {} help panel", if show { "Showing" } else { "Hiding" }));
                action
            }
            // Pass through other actions
            _ => action,
        }
    }

    fn start_background_sync(&mut self) {
        let sync_service = self.sync_service.clone();
        let task_id = self.task_manager.spawn_sync(sync_service);
        self.active_sync_task = Some(task_id);
    }

    /// Spawn a generic task operation (now with actual API calls and data refresh)
    fn spawn_task_operation(&mut self, operation_name: String, task_info: String) {
        let description = format!("{}: {}", operation_name, task_info);
        let op_name = operation_name.clone();
        let sync_service = self.sync_service.clone();
        self.logger
            .log(format!("Background: Spawning task operation '{}'", description));

        let _task_id = self.task_manager.spawn_task_operation(
            move || async move {
                let result = match op_name.as_str() {
                    "Complete task" => match sync_service.complete_task(&task_info).await {
                        Ok(()) => Ok(format!("✅ Task completed: {}", task_info)),
                        Err(e) => Err(format!("❌ Failed to complete task: {}", e)),
                    },
                    "Delete task" => match sync_service.delete_task(&task_info).await {
                        Ok(()) => Ok(format!("✅ Task deleted: {}", task_info)),
                        Err(e) => Err(format!("❌ Failed to delete task: {}", e)),
                    },
                    "Cycle priority" => {
                        // task_info format: "task_id|new_priority"
                        if let Some((task_id, priority_str)) = task_info.split_once('|') {
                            if let Ok(priority) = priority_str.parse::<i32>() {
                                match sync_service.update_task_priority(task_id, priority).await {
                                    Ok(()) => Ok(format!("✅ Task priority updated to P{}: {}", priority, task_id)),
                                    Err(e) => Err(format!("❌ Failed to update task priority: {}", e)),
                                }
                            } else {
                                Err("❌ Invalid priority value format".to_string())
                            }
                        } else {
                            Err("❌ Invalid task priority info format".to_string())
                        }
                    }
                    "Set task due today" => {
                        // task_info format: "task_id|today"
                        if let Some((task_id, _)) = task_info.split_once('|') {
                            let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                            match sync_service.update_task_due_date(task_id, Some(&today)).await {
                                Ok(()) => Ok(format!("✅ Task due date set to today: {}", task_id)),
                                Err(e) => Err(format!("❌ Failed to set task due date: {}", e)),
                            }
                        } else {
                            Err("❌ Invalid task info format for setting due date".to_string())
                        }
                    }
                    "Set task due tomorrow" => {
                        // task_info format: "task_id|tomorrow"
                        if let Some((task_id, _)) = task_info.split_once('|') {
                            let tomorrow =
                                (chrono::Utc::now() + chrono::Duration::days(1)).format("%Y-%m-%d").to_string();
                            match sync_service.update_task_due_date(task_id, Some(&tomorrow)).await {
                                Ok(()) => Ok(format!("✅ Task due date set to tomorrow: {}", task_id)),
                                Err(e) => Err(format!("❌ Failed to set task due date: {}", e)),
                            }
                        } else {
                            Err("❌ Invalid task info format for setting due date".to_string())
                        }
                    }
                    "Set task due next week" => {
                        // task_info format: "task_id|next_week"
                        if let Some((task_id, _)) = task_info.split_once('|') {
                            let today = chrono::Utc::now().date_naive();
                            let next_monday = crate::utils::datetime::next_weekday(today, chrono::Weekday::Mon);
                            let next_monday_str = crate::utils::datetime::format_ymd(next_monday);
                            match sync_service.update_task_due_date(task_id, Some(&next_monday_str)).await {
                                Ok(()) => Ok(format!("✅ Task due date set to next Monday: {}", task_id)),
                                Err(e) => Err(format!("❌ Failed to set task due date: {}", e)),
                            }
                        } else {
                            Err("❌ Invalid task info format for setting due date".to_string())
                        }
                    }
                    "Set task due weekend" => {
                        // task_info format: "task_id|weekend"
                        if let Some((task_id, _)) = task_info.split_once('|') {
                            let today = chrono::Utc::now().date_naive();
                            let next_saturday = crate::utils::datetime::next_weekday(today, chrono::Weekday::Sat);
                            let next_saturday_str = crate::utils::datetime::format_ymd(next_saturday);
                            match sync_service.update_task_due_date(task_id, Some(&next_saturday_str)).await {
                                Ok(()) => Ok(format!("✅ Task due date set to next Saturday: {}", task_id)),
                                Err(e) => Err(format!("❌ Failed to set task due date: {}", e)),
                            }
                        } else {
                            Err("❌ Invalid task info format for setting due date".to_string())
                        }
                    }
                    "Create task" => {
                        // task_info format: "content|project_id" or just "content" for inbox
                        if let Some((content, project_id)) = task_info.split_once('|') {
                            // Task has a specific project
                            match sync_service.create_task(content, Some(project_id)).await {
                                Ok(()) => Ok(format!("✅ Task created in project: {}", content)),
                                Err(e) => Err(format!("❌ Failed to create task: {}", e)),
                            }
                        } else {
                            // Task goes to inbox (no project_id)
                            match sync_service.create_task(&task_info, None).await {
                                Ok(()) => Ok(format!("✅ Task created in inbox: {}", task_info)),
                                Err(e) => Err(format!("❌ Failed to create task: {}", e)),
                            }
                        }
                    }
                    "Edit task" => {
                        // task_info format: "task_id: new_content"
                        if let Some((task_id, content)) = task_info.split_once(": ") {
                            match sync_service.update_task_content(task_id, content).await {
                                Ok(()) => Ok(format!("✅ Task updated: {}", task_id)),
                                Err(e) => Err(format!("❌ Failed to update task: {}", e)),
                            }
                        } else {
                            Err("❌ Invalid task edit format".to_string())
                        }
                    }
                    "Create project" => {
                        // project_info format: "name|parent_id" or just "name" for root project
                        if let Some((name, parent_id)) = task_info.split_once('|') {
                            // Project has a parent
                            match sync_service.create_project(name, Some(parent_id)).await {
                                Ok(()) => Ok(format!("✅ Project created with parent: {}", name)),
                                Err(e) => Err(format!("❌ Failed to create project: {}", e)),
                            }
                        } else {
                            // Root project (no parent)
                            match sync_service.create_project(&task_info, None).await {
                                Ok(()) => Ok(format!("✅ Root project created: {}", task_info)),
                                Err(e) => Err(format!("❌ Failed to create project: {}", e)),
                            }
                        }
                    }
                    "Delete project" => match sync_service.delete_project(&task_info).await {
                        Ok(()) => Ok(format!("✅ Project deleted: {}", task_info)),
                        Err(e) => Err(format!("❌ Failed to delete project: {}", e)),
                    },
                    "Delete label" => match sync_service.delete_label(&task_info).await {
                        Ok(()) => Ok(format!("✅ Label deleted: {}", task_info)),
                        Err(e) => Err(format!("❌ Failed to delete label: {}", e)),
                    },
                    "Create label" => match sync_service.create_label(&task_info, None).await {
                        Ok(()) => Ok(format!("✅ Label created: {}", task_info)),
                        Err(e) => Err(format!("❌ Failed to create label: {}", e)),
                    },
                    "Edit project" => {
                        // task_info format: "project_id: new_name"
                        if let Some((project_id, name)) = task_info.split_once(": ") {
                            match sync_service.update_project_content(project_id, name).await {
                                Ok(()) => Ok(format!("✅ Project updated: {}", project_id)),
                                Err(e) => Err(format!("❌ Failed to update project: {}", e)),
                            }
                        } else {
                            Err("❌ Invalid project edit format".to_string())
                        }
                    }
                    "Edit label" => {
                        // task_info format: "label_id: new_name"
                        if let Some((label_id, name)) = task_info.split_once(": ") {
                            match sync_service.update_label_content(label_id, name).await {
                                Ok(()) => Ok(format!("✅ Label updated: {}", label_id)),
                                Err(e) => Err(format!("❌ Failed to update label: {}", e)),
                            }
                        } else {
                            Err("❌ Invalid label edit format".to_string())
                        }
                    }
                    _ => Err(format!("❌ Unknown operation: {}", op_name)),
                };

                result.map_err(|e| anyhow::anyhow!(e))
            },
            description,
        );
    }

    fn update_data_from_sync(&mut self, status: SyncStatus) {
        // Only proceed if sync was successful
        if matches!(status, SyncStatus::Success) {
            // Schedule a data fetch task
            self.schedule_data_fetch();
        }
    }

    /// Schedule a background task to fetch data after sync completion
    fn schedule_data_fetch(&mut self) {
        let _task_id = self
            .task_manager
            .spawn_data_load(self.sync_service.clone(), self.state.sidebar_selection.clone());
    }

    /// Process background actions from task manager
    pub fn process_background_actions(&mut self) -> Vec<Action> {
        let mut actions = Vec::new();

        // Process all available background actions
        while let Ok(action) = self.background_action_rx.try_recv() {
            self.logger.log(format!("Background: Received action {:?}", action));
            actions.push(action);
        }

        // Clean up finished tasks
        let completed_tasks = self.task_manager.cleanup_finished_tasks();
        if !completed_tasks.is_empty() {
            self.logger.log(format!(
                "Background: Cleaned up {} finished tasks",
                completed_tasks.len()
            ));
        }

        actions
    }

    /// Check if any background operations are running
    pub fn is_busy(&self) -> bool {
        self.task_manager.task_count() > 0
    }

    /// Process an event through the component hierarchy
    pub async fn handle_event(&mut self, event_type: EventType) -> anyhow::Result<()> {
        let action = match event_type {
            EventType::Mouse(mouse) => {
                // Handle mouse events - assume sidebar is left 30 columns
                if !self.dialog.is_visible() && mouse.column < 30 {
                    let sidebar_area = Rect::new(0, 0, 30, 50); // Approximate sidebar area
                    self.sidebar.handle_mouse(mouse, sidebar_area)
                } else {
                    Action::None
                }
            }
            EventType::Key(key) => {
                // Route keyboard events to components or handle globally
                if self.dialog.is_visible() {
                    // Dialog has priority when visible
                    self.dialog.handle_key_events(key)
                } else {
                    // Try sidebar first (for J/K navigation)
                    let sidebar_action = self.sidebar.handle_key_events(key);

                    if !matches!(sidebar_action, Action::None) {
                        sidebar_action
                    } else {
                        // Then try task list (for j/k and other task operations)
                        let task_list_action = self.task_list.handle_key_events(key);

                        if !matches!(task_list_action, Action::None) {
                            task_list_action
                        } else {
                            // Finally try global keys
                            self.handle_global_key(key)
                        }
                    }
                }
            }
            EventType::Resize(_, _) => {
                // Handle terminal resize
                Action::None
            }
            EventType::Tick => {
                // Periodic updates
                Action::None
            }
            EventType::Render => {
                // Render updates
                Action::None
            }
            EventType::Other => Action::None,
        };

        // Process action through component hierarchy
        let action = self.dialog.update(action);
        let action = self.sidebar.update(action);
        let action = self.task_list.update(action);

        // Handle app-level actions
        let _final_action = self.handle_app_action(action).await;

        // Update component data after any changes
        self.sync_component_data();

        Ok(())
    }
}

impl Component for AppComponent {
    fn handle_key_events(&mut self, key: KeyEvent) -> Action {
        // This shouldn't be called directly - use handle_event instead
        self.handle_global_key(key)
    }

    fn update(&mut self, action: Action) -> Action {
        // Process through component hierarchy
        let action = self.dialog.update(action);
        let action = self.sidebar.update(action);

        // Return for app-level handling
        self.task_list.update(action)
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        // Create layout: sidebar (1/3 or 30 max) | task list (remainder)
        let sidebar_width = (rect.width / 3).min(30);
        let main_chunks = Layout::horizontal([Constraint::Length(sidebar_width), Constraint::Min(0)]).split(rect);

        // Render components
        self.sidebar.render(f, main_chunks[0]);
        self.task_list.render(f, main_chunks[1]);

        // Render sync status if syncing or loading
        if self.state.loading || self.is_syncing() {
            AppComponent::render_sync_status_impl(self, f, rect);
        }

        // Render dialog on top if visible (includes help dialog)
        if self.dialog.is_visible() {
            self.dialog.render(f, rect);
        }
    }
}

impl AppComponent {
    /// Render sync status indicator
    fn render_sync_status_impl(&self, f: &mut Frame, rect: Rect) {
        use ratatui::{
            layout::{Alignment, Constraint, Layout},
            style::{Color, Style},
            text::{Line, Span},
            widgets::{Block, Borders, Clear, Paragraph},
        };

        // Calculate centered area for the sync indicator
        let popup_area = {
            let popup_layout =
                Layout::vertical([Constraint::Percentage(40), Constraint::Min(3), Constraint::Percentage(40)])
                    .split(rect);

            Layout::horizontal([Constraint::Percentage(30), Constraint::Min(30), Constraint::Percentage(30)])
                .split(popup_layout[1])[1]
        };

        let title = if self.state.loading {
            "Loading data"
        } else {
            "Syncing with Todoist"
        };

        let spinner = "⟳";
        let content = Paragraph::new(Line::from(Span::styled(
            format!("{} {}...", spinner, title),
            Style::default().fg(Color::Yellow),
        )))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).style(Style::default().fg(Color::Yellow)));

        f.render_widget(Clear, popup_area);
        f.render_widget(content, popup_area);
    }
}
