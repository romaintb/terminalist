use crate::config::Config;
use crate::constants::*;
use crate::entities::{label, project, section, task};
use crate::sync::{SyncService, SyncStatus};
use crate::ui::components::{DialogComponent, SidebarComponent, TaskListComponent};
use crate::ui::core::{
    actions::{Action, DialogType, NavigationCounts, TaskDueDate},
    event_handler::EventType,
    operations::{LabelOperation, Operation, ProjectOperation, TaskOperation},
    task_manager::{TaskId, TaskManager},
    Component,
};
use crate::ui::core::{SidebarSelection, ViewSnapshot};
use crate::utils::datetime;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use log::info;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    Frame,
};
use tokio::sync::mpsc;
use uuid::Uuid;

/// Application state separate from UI concerns
#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub projects: Vec<project::Model>,
    pub tasks: Vec<task::Model>,
    pub all_tasks: Vec<task::Model>,
    pub labels: Vec<label::Model>,
    pub sections: Vec<section::Model>,
    pub sidebar_selection: SidebarSelection,
    pub loading: bool,
    pub error_message: Option<String>,
    pub info_message: Option<String>,
    pub show_help: bool,
    pub navigation_counts: NavigationCounts,
    /// didnt we just got rid of custom scrolling ?
    pub help_scroll_offset: usize,
}

impl AppState {
    /// Update all data at once
    pub fn update_data(
        &mut self,
        projects: Vec<project::Model>,
        labels: Vec<label::Model>,
        sections: Vec<section::Model>,
        tasks: Vec<task::Model>,
        all_tasks: Vec<task::Model>,
        navigation_counts: NavigationCounts,
    ) {
        self.projects = projects;
        self.labels = labels;
        self.sections = sections;
        self.tasks = tasks;
        self.all_tasks = all_tasks;
        self.navigation_counts = navigation_counts;
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

    // Configuration
    config: Config,

    // Simple UI state
    should_quit: bool,
    active_sync_task: Option<TaskId>,
    is_initial_sync: bool,
    next_load_generation: u64,
    latest_requested_generation: u64,
    latest_applied_generation: u64,

    // Layout state
    sidebar_visible: bool,
    sidebar_width: u16,
    sidebar_width_override: Option<u16>,
    resizing_sidebar: bool,
    active_pane: ActivePane,
    screen_width: u16,
    screen_height: u16,
}

#[derive(Clone, Copy, PartialEq)]
enum ActivePane {
    Navigation,
    Tasks,
}

impl AppComponent {
    pub fn new(sync_service: SyncService, config: Config) -> Self {
        let sidebar = SidebarComponent::new();
        let task_list = TaskListComponent::new();
        let (task_manager, background_action_rx) = TaskManager::new();
        let sidebar_width_override =
            (config.ui.sidebar_width != SIDEBAR_DEFAULT_WIDTH).then_some(config.ui.sidebar_width);

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
            sidebar_visible: config.ui.sidebar_visible,
            config,
            should_quit: false,
            active_sync_task: None,
            is_initial_sync: false,
            next_load_generation: 1,
            latest_requested_generation: 0,
            latest_applied_generation: 0,
            sidebar_width: 30, // Default width
            sidebar_width_override,
            resizing_sidebar: false,
            active_pane: ActivePane::Tasks,
            screen_width: 100, // Default width
            screen_height: 50, // Default height
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

    /// Trigger initial sync on startup (unless in debug mode)
    pub fn trigger_initial_sync(&mut self) {
        if self.sync_service.is_debug_mode() {
            info!("AppComponent: Skipping initial sync (debug mode)");
            // In debug mode, just load existing data from database
            self.is_initial_sync = true;
            self.schedule_initial_data_fetch();
            self.is_initial_sync = false;
        } else {
            info!("AppComponent: Loading cached data before initial sync");
            if self.active_sync_task.is_none() {
                self.is_initial_sync = true;
                self.schedule_initial_data_fetch();
                self.start_background_sync();
                // A successful sync refreshes the view again. A failed sync leaves the
                // already-scheduled cached snapshot visible.
                info!("AppComponent: Cached data load and initial sync scheduled");
            }
        }
    }

    /// Set initial sidebar selection based on config
    fn set_initial_sidebar_selection(&mut self) {
        let selection = match self.config.ui.default_project.as_str() {
            "inbox" => {
                // Find inbox project
                if let Some(inbox) = self.state.projects.iter().find(|p| p.is_inbox_project) {
                    SidebarSelection::Project(inbox.uuid)
                } else {
                    SidebarSelection::Today
                }
            }
            "today" => SidebarSelection::Today,
            "agenda" => SidebarSelection::Agenda,
            "tomorrow" => SidebarSelection::Tomorrow,
            "upcoming" => SidebarSelection::Upcoming,
            project_id_or_name => {
                // Try to find project by ID first (parse as UUID), then by name
                if let Ok(uuid) = Uuid::parse_str(project_id_or_name) {
                    if let Some(project) = self.state.projects.iter().find(|p| p.uuid == uuid) {
                        SidebarSelection::Project(project.uuid)
                    } else if let Some(project) = self.state.projects.iter().find(|p| p.name == project_id_or_name) {
                        SidebarSelection::Project(project.uuid)
                    } else {
                        SidebarSelection::Today
                    }
                } else if let Some(project) = self.state.projects.iter().find(|p| p.name == project_id_or_name) {
                    SidebarSelection::Project(project.uuid)
                } else {
                    SidebarSelection::Today
                }
            }
        };

        self.state.sidebar_selection = selection;
        info!(
            "AppComponent: Set initial sidebar selection to {:?}",
            self.state.sidebar_selection
        );
    }

    /// Update all components with current data
    fn sync_component_data(&mut self) {
        // Update task list
        self.task_list.update_display_config(self.config.display.clone());
        self.task_list.update_all_tasks(self.state.all_tasks.clone());
        self.task_list.update_data(
            self.state.tasks.clone(),
            self.state.sections.clone(),
            self.state.projects.clone(),
            self.state.labels.clone(),
            self.state.sidebar_selection.clone(),
        );

        // Update sidebar after the task list so its selected count matches rendered task rows.
        self.sidebar.selection = self.state.sidebar_selection.clone();
        self.sidebar.update_data(
            self.state.projects.clone(),
            self.state.labels.clone(),
            self.state.navigation_counts.clone(),
            self.task_list.visible_incomplete_task_count(),
        );

        // Update dialog
        self.dialog.update_display_config(self.config.display.clone());
        self.dialog.update_data_with_tasks(
            self.state.projects.clone(),
            self.state.labels.clone(),
            self.state.tasks.clone(),
        );
        self.dialog.set_sync_service(self.sync_service.clone());
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
            KeyCode::Char('b') => {
                info!("Global key: 'b' - toggling sidebar visibility");
                Action::ToggleSidebar
            }
            KeyCode::Char('q') => {
                info!("Global key: 'q' - quitting application");
                Action::Quit
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                info!("Global key: Ctrl+C - quitting application");
                Action::Quit
            }
            KeyCode::Char('?') | KeyCode::Char('h') => {
                info!("Global key: '?' or 'h' - opening help dialog");
                Action::ShowDialog(DialogType::Help)
            }
            KeyCode::Char('G') => {
                info!("Global key: 'G' - opening logs dialog");
                Action::ShowDialog(DialogType::Logs)
            }
            KeyCode::Char('A') => {
                info!("Global key: 'A' - opening project creation dialog");
                Action::ShowDialog(DialogType::ProjectCreation)
            }
            KeyCode::Char('D') => {
                // Delete current project (only if a project is selected)
                match &self.state.sidebar_selection {
                    SidebarSelection::Project(uuid) => {
                        if let Some(project) = self.state.projects.iter().find(|project| project.uuid == *uuid) {
                            info!(
                                "Global key: 'D' - deleting project '{}' (ID: {})",
                                project.name, project.uuid
                            );
                            Action::ShowDialog(DialogType::DeleteConfirmation {
                                item_type: "project".to_string(),
                                item_uuid: project.uuid,
                            })
                        } else {
                            info!("Global key: 'D' - no project selected (invalid index)");
                            Action::ShowDialog(DialogType::Error("No project selected to delete".to_string()))
                        }
                    }
                    SidebarSelection::Today => {
                        info!("Global key: 'D' - cannot delete Today view");
                        Action::ShowDialog(DialogType::Info(UI_CANNOT_DELETE_TODAY_VIEW.to_string()))
                    }
                    SidebarSelection::Agenda => {
                        Action::ShowDialog(DialogType::Info("Cannot delete the Agenda view".to_string()))
                    }
                    SidebarSelection::Tomorrow => {
                        info!("Global key: 'D' - cannot delete Tomorrow view");
                        Action::ShowDialog(DialogType::Info("Cannot delete the Tomorrow view".to_string()))
                    }
                    SidebarSelection::Upcoming => {
                        info!("Global key: 'D' - cannot delete Upcoming view");
                        Action::ShowDialog(DialogType::Info("Cannot delete the Upcoming view".to_string()))
                    }
                    SidebarSelection::Trash => Action::ShowDialog(DialogType::EmptyTrashConfirmation {
                        count: self.state.all_tasks.iter().filter(|task| task.is_deleted).count(),
                    }),
                    SidebarSelection::Label(uuid) => {
                        if let Some(label) = self.state.labels.iter().find(|label| label.uuid == *uuid) {
                            info!("Global key: 'D' - deleting label '{}' (ID: {})", label.name, label.uuid);
                            Action::ShowDialog(DialogType::DeleteConfirmation {
                                item_type: "label".to_string(),
                                item_uuid: label.uuid,
                            })
                        } else {
                            info!("Global key: 'D' - no label selected (invalid index)");
                            Action::ShowDialog(DialogType::Error("No label selected to delete".to_string()))
                        }
                    }
                }
            }
            KeyCode::Char('E') => {
                // Edit current sidebar selection (project or label)
                match &self.state.sidebar_selection {
                    SidebarSelection::Project(uuid) => {
                        if let Some(project) = self.state.projects.iter().find(|project| project.uuid == *uuid) {
                            info!(
                                "Global key: 'E' - editing project '{}' (ID: {})",
                                project.name, project.uuid
                            );
                            Action::ShowDialog(DialogType::ProjectEdit {
                                project_uuid: project.uuid,
                                name: project.name.clone(),
                            })
                        } else {
                            info!("Global key: 'E' - no project selected (invalid index)");
                            Action::ShowDialog(DialogType::Error("No project selected to edit".to_string()))
                        }
                    }
                    SidebarSelection::Today => {
                        info!("Global key: 'E' - cannot edit Today view");
                        Action::ShowDialog(DialogType::Info("Cannot edit the Today view".to_string()))
                    }
                    SidebarSelection::Agenda => {
                        Action::ShowDialog(DialogType::Info("Cannot edit the Agenda view".to_string()))
                    }
                    SidebarSelection::Tomorrow => {
                        info!("Global key: 'E' - cannot edit Tomorrow view");
                        Action::ShowDialog(DialogType::Info("Cannot edit the Tomorrow view".to_string()))
                    }
                    SidebarSelection::Upcoming => {
                        info!("Global key: 'E' - cannot edit Upcoming view");
                        Action::ShowDialog(DialogType::Info("Cannot edit the Upcoming view".to_string()))
                    }
                    SidebarSelection::Trash => {
                        Action::ShowDialog(DialogType::Info("Cannot edit the Trash view".to_string()))
                    }
                    SidebarSelection::Label(uuid) => {
                        if let Some(label) = self.state.labels.iter().find(|label| label.uuid == *uuid) {
                            info!("Global key: 'E' - editing label '{}' (ID: {})", label.name, label.uuid);
                            Action::ShowDialog(DialogType::LabelEdit {
                                label_uuid: label.uuid,
                                name: label.name.clone(),
                            })
                        } else {
                            info!("Global key: 'E' - no label selected (invalid index)");
                            Action::ShowDialog(DialogType::Error("No label selected to edit".to_string()))
                        }
                    }
                }
            }
            KeyCode::Char('r') => {
                info!("Global key: 'r' - starting manual sync");
                Action::StartSync
            }
            KeyCode::Char('R') => {
                if self.sync_service.is_debug_mode() {
                    info!("Global key: 'R' - refreshing local data (debug mode)");
                    Action::RefreshLocalData
                } else {
                    Action::None
                }
            }
            KeyCode::Char('/') => {
                info!("Global key: '/' - opening task search dialog");
                Action::ShowDialog(DialogType::TaskSearch)
            }
            KeyCode::Char('t') => {
                // Set task due date to today
                if let Some(task) = self.task_list.get_selected_task() {
                    info!("Global key: 't' - setting task '{}' due today", task.content);
                    Action::SetTaskDueToday(task.uuid)
                } else {
                    info!("Global key: 't' - no task selected");
                    Action::ShowDialog(DialogType::Info(UI_NO_TASK_SELECTED_DUE_DATE.to_string()))
                }
            }
            KeyCode::Char('T') => {
                // Set task due date to tomorrow
                if let Some(task) = self.task_list.get_selected_task() {
                    info!("Global key: 'T' - setting task '{}' due tomorrow", task.content);
                    Action::SetTaskDueTomorrow(task.uuid)
                } else {
                    info!("Global key: 'T' - no task selected");
                    Action::ShowDialog(DialogType::Info(UI_NO_TASK_SELECTED_DUE_DATE.to_string()))
                }
            }
            KeyCode::Char('w') => {
                // Set task due date to next week (Monday)
                if let Some(task) = self.task_list.get_selected_task() {
                    info!("Global key: 'w' - setting task '{}' due next week", task.content);
                    Action::SetTaskDueNextWeek(task.uuid)
                } else {
                    info!("Global key: 'w' - no task selected");
                    Action::ShowDialog(DialogType::Info(UI_NO_TASK_SELECTED_DUE_DATE.to_string()))
                }
            }
            KeyCode::Char('W') => {
                // Set task due date to weekend (Saturday)
                if let Some(task) = self.task_list.get_selected_task() {
                    info!("Global key: 'W' - setting task '{}' due weekend", task.content);
                    Action::SetTaskDueWeekEnd(task.uuid)
                } else {
                    info!("Global key: 'W' - no task selected");
                    Action::ShowDialog(DialogType::Info(UI_NO_TASK_SELECTED_DUE_DATE.to_string()))
                }
            }
            KeyCode::Esc => {
                if self.dialog.is_visible() {
                    info!("Global key: Esc - closing dialog");
                    Action::HideDialog
                } else {
                    info!("Global key: Esc - quitting application");
                    Action::Quit
                }
            }
            _ => Action::None,
        }
    }

    /// Handle app-level actions that require business logic
    pub async fn handle_app_action(&mut self, action: Action) -> Action {
        match action {
            Action::ToggleSidebar => {
                self.sidebar_visible = !self.sidebar_visible;
                Action::None
            }
            Action::Quit => {
                self.should_quit = true;
                Action::None
            }
            Action::StartSync => {
                if self.active_sync_task.is_none() {
                    info!("Starting background sync");
                    self.state.loading = true;
                    self.start_background_sync();
                } else {
                    info!("Sync already in progress, ignoring");
                }
                Action::None
            }
            Action::RefreshLocalData => {
                info!("Refreshing local data from database (debug mode)");
                // Schedule a data fetch directly from local storage without API sync
                self.schedule_data_fetch();
                Action::None
            }
            Action::SyncCompleted(status) => {
                info!("Sync: Completed with status {:?}", status);
                self.active_sync_task = None;
                self.state.loading = false;

                match status {
                    SyncStatus::Success => {
                        self.update_data_from_sync(SyncStatus::Success);
                        self.sync_component_data();
                        self.state.info_message = Some(SUCCESS_SYNC_COMPLETED.to_string());
                        info!("Sync: Showing completion info dialog");
                        Action::ShowDialog(DialogType::Info(self.state.info_message.clone().unwrap()))
                    }
                    SyncStatus::Error { message } => {
                        self.is_initial_sync = false;
                        self.state.error_message = Some(message);
                        Action::ShowDialog(DialogType::Error(self.state.error_message.clone().unwrap_or_default()))
                    }
                    SyncStatus::Idle | SyncStatus::InProgress => Action::None,
                }
            }
            Action::SyncFailed(error) => {
                info!("Sync: Failed with error: {}", error);
                self.active_sync_task = None;
                self.state.loading = false;
                self.is_initial_sync = false; // Reset flag on failure
                self.state.error_message = Some(error);
                Action::ShowDialog(DialogType::Error(self.state.error_message.clone().unwrap_or_default()))
            }
            Action::ShowDialog(ref dialog_type) => {
                info!("Dialog: Showing dialog {:?}", dialog_type);
                // Dialog component will handle the actual dialog setup
                action
            }
            Action::HideDialog => {
                info!("Dialog: Hiding current dialog");
                // Dialog component will handle hiding
                action
            }
            Action::NavigateToSidebar(selection) => {
                // Create a more detailed log message with names
                let selection_desc = match &selection {
                    SidebarSelection::Today => "Today".to_string(),
                    SidebarSelection::Agenda => "Agenda".to_string(),
                    SidebarSelection::Tomorrow => "Tomorrow".to_string(),
                    SidebarSelection::Upcoming => "Upcoming".to_string(),
                    SidebarSelection::Trash => "Trash".to_string(),
                    SidebarSelection::Project(uuid) => {
                        if let Some(project) = self.state.projects.iter().find(|project| project.uuid == *uuid) {
                            format!("Project({}) '{}'", uuid, project.name)
                        } else {
                            format!("Project({}) [unknown]", uuid)
                        }
                    }
                    SidebarSelection::Label(uuid) => {
                        if let Some(label) = self.state.labels.iter().find(|label| label.uuid == *uuid) {
                            format!("Label({}) '{}'", uuid, label.name)
                        } else {
                            format!("Label({}) [unknown]", uuid)
                        }
                    }
                };

                info!("Navigation: Sidebar selection changed to {}", selection_desc);
                self.state.sidebar_selection = selection.clone();
                // Reload data for the new selection
                self.schedule_data_fetch();
                info!("Navigation: Scheduled data fetch for new selection");
                Action::None
            }
            // Task operations with background execution
            Action::CreateTask {
                content,
                project_uuid,
                due_date,
                label_uuid,
            } => {
                let project_desc = match &project_uuid {
                    Some(uuid) => format!(" in project {}", uuid),
                    None => " in inbox".to_string(),
                };
                info!("Task: Creating task with content '{}'{}", content, project_desc);

                self.spawn_operation(Operation::Task(TaskOperation::Create {
                    content,
                    project_uuid,
                    due_date,
                    label_uuid,
                }));
                Action::None
            }
            Action::ToggleTasks(tasks) => {
                let count = tasks.len();
                let sync_service = self.sync_service.clone();
                if let Some(task_uuid) = Self::single_task_completion(&tasks) {
                    if self.task_manager.has_pending_operation_for_task(&task_uuid) {
                        info!("Task: Completion already pending for task {}", task_uuid);
                        return Action::None;
                    }
                    self.task_manager.spawn_non_blocking_task_operation(
                        task_uuid,
                        move || async move {
                            sync_service.complete_task(&task_uuid).await?;
                            Ok("Completed task".to_string())
                        },
                        "Complete task".to_string(),
                    );
                } else {
                    self.task_manager.spawn_task_operation(
                        move || async move {
                            for (task_uuid, should_restore) in tasks {
                                if should_restore {
                                    sync_service.restore_task(&task_uuid).await?;
                                } else {
                                    sync_service.complete_task(&task_uuid).await?;
                                }
                            }
                            Ok(format!("Updated {} task(s)", count))
                        },
                        format!("Toggle {} selected task(s)", count),
                    );
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
                    info!("Task: Cycling priority for task {}", task_desc);
                    self.spawn_operation(Operation::Task(TaskOperation::CyclePriority {
                        task_uuid: task_id,
                        priority: new_priority,
                    }));
                } else {
                    info!("Task: Cannot cycle priority - task {} not found", task_id);
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
                info!("Task: Deleting task {}", task_desc);
                self.spawn_operation(Operation::Task(TaskOperation::Delete(task_id)));
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
                info!("Task: Setting due date to today for task {}", task_desc);
                self.spawn_operation(Operation::Task(TaskOperation::SetDueDate {
                    task_uuid: task_id,
                    due_date: TaskDueDate::Today,
                }));
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
                info!("Task: Setting due date to tomorrow for task {}", task_desc);
                self.spawn_operation(Operation::Task(TaskOperation::SetDueDate {
                    task_uuid: task_id,
                    due_date: TaskDueDate::Tomorrow,
                }));
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
                info!("Task: Setting due date to next week for task {}", task_desc);
                self.spawn_operation(Operation::Task(TaskOperation::SetDueDate {
                    task_uuid: task_id,
                    due_date: TaskDueDate::NextWeek,
                }));
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
                info!("Task: Setting due date to weekend for task {}", task_desc);
                self.spawn_operation(Operation::Task(TaskOperation::SetDueDate {
                    task_uuid: task_id,
                    due_date: TaskDueDate::Weekend,
                }));
                Action::None
            }
            Action::SetTasksDueDate { task_ids, due_date } => {
                let count = task_ids.len();
                let operation = match due_date {
                    TaskDueDate::None => "Unscheduling",
                    TaskDueDate::Today => "Scheduling for today",
                    TaskDueDate::Tomorrow => "Scheduling for tomorrow",
                    TaskDueDate::NextWeek => "Scheduling for next week",
                    TaskDueDate::Weekend => "Scheduling for the weekend",
                };
                let sync_service = self.sync_service.clone();
                self.task_manager.spawn_task_operation(
                    move || async move {
                        let due_date_value = match due_date {
                            TaskDueDate::None => None,
                            TaskDueDate::Today => Some(datetime::format_today()),
                            TaskDueDate::Tomorrow => Some(datetime::format_date_with_offset(1)),
                            TaskDueDate::NextWeek => {
                                let today = chrono::Local::now().date_naive();
                                let date = datetime::next_weekday(today, chrono::Weekday::Mon);
                                Some(datetime::format_ymd(date))
                            }
                            TaskDueDate::Weekend => {
                                let today = chrono::Local::now().date_naive();
                                let date = datetime::next_weekday(today, chrono::Weekday::Sat);
                                Some(datetime::format_ymd(date))
                            }
                        };

                        for task_uuid in task_ids {
                            sync_service.update_task_due_date(&task_uuid, due_date_value.as_deref()).await?;
                        }
                        Ok(format!("Updated due date for {} task(s)", count))
                    },
                    format!("{operation} {count} task(s)"),
                );
                Action::None
            }
            Action::SetTaskDueTime {
                task_uuid,
                due_datetime,
            } => {
                let sync_service = self.sync_service.clone();
                self.task_manager.spawn_task_operation(
                    move || async move {
                        sync_service.update_task_due_datetime(&task_uuid, &due_datetime).await?;
                        Ok("Set task time".to_string())
                    },
                    "Set task time".to_string(),
                );
                Action::None
            }
            Action::EditTask { task_uuid, content } => {
                info!("Task: Editing task UUID {} with new content '{}'", task_uuid, content);
                self.spawn_operation(Operation::Task(TaskOperation::Edit { task_uuid, content }));
                Action::None
            }
            Action::RestoreTask(task_id) => {
                info!("Task: Restoring task {}", task_id);
                self.spawn_operation(Operation::Task(TaskOperation::Restore(task_id)));
                Action::None
            }
            Action::EmptyTrash => {
                let sync_service = self.sync_service.clone();
                self.task_manager.spawn_task_operation(
                    move || async move {
                        let count = sync_service.empty_trash().await?;
                        Ok(format!("Permanently deleted {} task(s)", count))
                    },
                    "Empty trash".to_string(),
                );
                Action::None
            }
            Action::CreateProject { name, parent_uuid } => {
                let parent_desc = match &parent_uuid {
                    Some(uuid) => format!(" with parent {}", uuid),
                    None => "".to_string(),
                };
                info!("Project: Creating project '{}'{}", name, parent_desc);

                self.spawn_operation(Operation::Project(ProjectOperation::Create { name, parent_uuid }));
                Action::None
            }
            Action::DeleteProject(project_id) => {
                // Find project name for better logging
                let project_desc = if let Some(project) = self.state.projects.iter().find(|p| p.uuid == project_id) {
                    format!("ID {} '{}'", project_id, project.name)
                } else {
                    format!("ID {} [unknown]", project_id)
                };
                info!("Project: Deleting project {}", project_desc);
                self.spawn_operation(Operation::Project(ProjectOperation::Delete(project_id)));
                Action::None
            }
            Action::DeleteLabel(label_id) => {
                // Find label name for better logging
                let label_desc = if let Some(label) = self.state.labels.iter().find(|l| l.uuid == label_id) {
                    format!("ID {} '{}'", label_id, label.name)
                } else {
                    format!("ID {} [unknown]", label_id)
                };
                info!("Label: Deleting label {}", label_desc);
                self.spawn_operation(Operation::Label(LabelOperation::Delete(label_id)));
                Action::None
            }
            Action::CreateLabel { name } => {
                info!("Label: Creating label '{}'", name);
                self.spawn_operation(Operation::Label(LabelOperation::Create { name }));
                Action::None
            }
            Action::EditProject { project_uuid, name } => {
                // Find project name for better logging
                let project_desc = if let Some(project) = self.state.projects.iter().find(|p| p.uuid == project_uuid) {
                    format!("UUID {} '{}' -> '{}'", project_uuid, project.name, name)
                } else {
                    format!("UUID {} [unknown] -> '{}'", project_uuid, name)
                };
                info!("Project: Editing project {}", project_desc);
                self.spawn_operation(Operation::Project(ProjectOperation::Edit { project_uuid, name }));
                Action::None
            }
            Action::EditLabel { label_uuid, name } => {
                // Find label name for better logging
                let label_desc = if let Some(label) = self.state.labels.iter().find(|l| l.uuid == label_uuid) {
                    format!("UUID {} '{}' -> '{}'", label_uuid, label.name, name)
                } else {
                    format!("UUID {} [unknown] -> '{}'", label_uuid, name)
                };
                info!("Label: Editing label {}", label_desc);
                self.spawn_operation(Operation::Label(LabelOperation::Edit { label_uuid, name }));
                Action::None
            }
            Action::DataLoaded(snapshot) => {
                if snapshot.generation != self.latest_requested_generation
                    || snapshot.selection != self.state.sidebar_selection
                {
                    info!(
                        "Data: Ignoring stale generation {} for {:?}; latest is {} for {:?}",
                        snapshot.generation,
                        snapshot.selection,
                        self.latest_requested_generation,
                        self.state.sidebar_selection
                    );
                    return Action::None;
                }
                info!(
                    "Data: Applying generation {} with {} projects, {} labels, {} sections, {} tasks",
                    snapshot.generation,
                    snapshot.projects.len(),
                    snapshot.labels.len(),
                    snapshot.sections.len(),
                    snapshot.tasks.len()
                );
                let was_initial = snapshot.is_initial;
                let trash_became_empty =
                    snapshot.selection == SidebarSelection::Trash && snapshot.navigation_counts.trash == 0;
                self.apply_snapshot(*snapshot);
                if trash_became_empty {
                    self.state.sidebar_selection = SidebarSelection::Today;
                }
                self.sync_component_data();
                if was_initial {
                    self.set_initial_sidebar_selection();
                    self.schedule_data_fetch();
                } else if trash_became_empty {
                    self.schedule_data_fetch();
                }
                Action::None
            }
            Action::DataLoadFailed {
                generation,
                selection,
                message,
            } => {
                if generation != self.latest_requested_generation || selection != self.state.sidebar_selection {
                    info!("Data: Ignoring stale failure for generation {}", generation);
                    return Action::None;
                }
                self.state.loading = false;
                self.state.error_message = Some(message.clone());
                Action::ShowDialog(DialogType::Error(message))
            }
            Action::SearchTasks(query) => {
                info!("Search: Starting database search for '{}'", query);
                let sync_service = self.sync_service.clone();
                let _task_id = self.task_manager.spawn_task_search(sync_service, query);
                Action::None
            }
            Action::SearchResultsLoaded { query, results } => {
                info!("Search: Loaded {} results for query '{}'", results.len(), query);
                // Update dialog with search results
                self.dialog.update_search_results(&query, results);
                Action::None
            }
            Action::NextTask => {
                info!("Navigation: Next task (j/down)");
                action
            }
            Action::PreviousTask => {
                info!("Navigation: Previous task (k/up)");
                action
            }
            Action::RefreshData => {
                info!("Data: Refreshing UI data after task operation");
                // Schedule a data fetch to reload current view with updated data
                self.schedule_data_fetch();
                if matches!(self.dialog.dialog_type, Some(DialogType::TaskSearch)) {
                    let query = self.dialog.input_buffer.clone();
                    info!("Search: Refreshing active query '{}' after task operation", query);
                    self.task_manager.spawn_task_search(self.sync_service.clone(), query);
                }
                Action::None
            }
            // Help panel scrolling actions
            Action::HelpScrollUp => {
                if self.state.help_scroll_offset > 0 {
                    self.state.help_scroll_offset -= 1;
                }
                info!("Help: Scrolled up, offset now {}", self.state.help_scroll_offset);
                Action::None
            }
            Action::HelpScrollDown => {
                self.state.help_scroll_offset += 1;
                info!("Help: Scrolled down, offset now {}", self.state.help_scroll_offset);
                Action::None
            }
            Action::HelpScrollToTop => {
                self.state.help_scroll_offset = 0;
                info!("Help: Scrolled to top");
                Action::None
            }
            Action::HelpScrollToBottom => {
                // Set to a large value - dialog component will handle bounds checking
                self.state.help_scroll_offset = usize::MAX;
                info!("Help: Scrolled to bottom");
                Action::None
            }
            Action::ShowHelp(show) => {
                self.state.show_help = show;
                if !show {
                    // Reset scroll when hiding help
                    self.state.help_scroll_offset = 0;
                }
                info!("Help: {} help panel", if show { "Showing" } else { "Hiding" });
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

    fn single_task_completion(tasks: &[(Uuid, bool)]) -> Option<Uuid> {
        match tasks {
            [(task_uuid, false)] => Some(*task_uuid),
            _ => None,
        }
    }

    fn spawn_operation(&mut self, operation: Operation) {
        let description = operation.description();
        let completed_task_uuid = match &operation {
            Operation::Task(TaskOperation::Complete(task_uuid)) => Some(*task_uuid),
            _ => None,
        };
        let sync_service = self.sync_service.clone();
        info!("Background: Spawning typed operation '{}'", description);

        if let Some(task_uuid) = completed_task_uuid {
            self.task_manager.spawn_non_blocking_task_operation(
                task_uuid,
                move || async move { operation.execute(&sync_service).await },
                description,
            );
        } else {
            self.task_manager.spawn_task_operation(
                move || async move { operation.execute(&sync_service).await },
                description,
            );
        }
    }

    fn update_data_from_sync(&mut self, status: SyncStatus) {
        // Only proceed if sync was successful
        if matches!(status, SyncStatus::Success) {
            if self.is_initial_sync {
                // For initial sync, use initial data fetch which sets default selection
                self.schedule_initial_data_fetch();
                self.is_initial_sync = false;
            } else {
                // For manual refresh, use regular data fetch to maintain current selection
                self.schedule_data_fetch();
            }
        }
    }

    fn apply_snapshot(&mut self, snapshot: ViewSnapshot) {
        self.latest_applied_generation = snapshot.generation;
        self.state.loading = false;
        self.state.update_data(
            snapshot.projects,
            snapshot.labels,
            snapshot.sections,
            snapshot.tasks,
            snapshot.all_tasks,
            snapshot.navigation_counts,
        );
    }

    fn next_load_generation(&mut self) -> u64 {
        let generation = self.next_load_generation;
        self.next_load_generation += 1;
        self.latest_requested_generation = generation;
        self.state.loading = true;
        generation
    }

    /// Schedule a background task to fetch initial data after sync completion
    fn schedule_initial_data_fetch(&mut self) {
        let generation = self.next_load_generation();
        let _task_id = self.task_manager.spawn_data_load(
            self.sync_service.clone(),
            self.state.sidebar_selection.clone(),
            generation,
            true,
        );
    }

    /// Schedule a background task to fetch data after navigation or changes
    fn schedule_data_fetch(&mut self) {
        let generation = self.next_load_generation();
        let _task_id = self.task_manager.spawn_data_load(
            self.sync_service.clone(),
            self.state.sidebar_selection.clone(),
            generation,
            false,
        );
    }

    /// Process background actions from task manager
    pub fn process_background_actions(&mut self) -> Vec<Action> {
        let mut actions = Vec::new();

        // Process all available background actions
        while let Ok(action) = self.background_action_rx.try_recv() {
            info!("Background: Received action {:?}", action);
            actions.push(action);
        }

        // Clean up finished tasks
        let completed_tasks = self.task_manager.cleanup_finished_tasks();
        if !completed_tasks.is_empty() {
            let count = completed_tasks.len();
            info!("Background: Cleaned up {} finished tasks", count);
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
                if !self.dialog.is_visible() {
                    if self.resizing_sidebar {
                        if matches!(mouse.kind, crossterm::event::MouseEventKind::Drag(_)) {
                            self.sidebar_width_override = Some(
                                mouse
                                    .column
                                    .clamp(SIDEBAR_MIN_WIDTH, self.screen_width.saturating_sub(MAIN_AREA_MIN_WIDTH)),
                            );
                        }
                        if matches!(mouse.kind, crossterm::event::MouseEventKind::Up(_)) {
                            self.resizing_sidebar = false;
                        }
                        Action::None
                    } else if self.sidebar_visible
                        && mouse.column.abs_diff(self.sidebar_width) <= 1
                        && matches!(
                            mouse.kind,
                            crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left)
                        )
                    {
                        self.resizing_sidebar = true;
                        Action::None
                    } else if self.sidebar_visible && mouse.column < self.sidebar_width {
                        self.active_pane = ActivePane::Navigation;
                        // Mouse is in sidebar area
                        let sidebar_area = Rect::new(0, 0, self.sidebar_width, self.screen_height);
                        self.sidebar.handle_mouse(mouse, sidebar_area)
                    } else {
                        self.active_pane = ActivePane::Tasks;
                        // Mouse is in task list area - calculate proper width
                        let task_list_width = self.screen_width.saturating_sub(self.sidebar_width).max(1);
                        let task_list_area = Rect::new(self.sidebar_width, 0, task_list_width, self.screen_height);
                        self.task_list.handle_mouse(mouse, task_list_area)
                    }
                } else {
                    Action::None
                }
            }
            EventType::Key(key) => {
                // Route keyboard events to components or handle globally
                if self.dialog.is_visible() {
                    // Dialog has priority when visible
                    self.dialog.handle_key_events(key)
                } else if self.task_manager.has_blocking_work() {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('?') | KeyCode::Char('h') => self.handle_global_key(key),
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            self.handle_global_key(key)
                        }
                        _ => Action::None,
                    }
                } else if key.code == KeyCode::Left && self.sidebar_visible {
                    self.active_pane = ActivePane::Navigation;
                    Action::None
                } else if key.code == KeyCode::Right {
                    self.active_pane = ActivePane::Tasks;
                    Action::None
                } else {
                    let pane_action = match self.active_pane {
                        ActivePane::Navigation => self.sidebar.handle_key_events(key),
                        ActivePane::Tasks => self.task_list.handle_key_events(key),
                    };
                    if matches!(pane_action, Action::None) {
                        self.handle_global_key(key)
                    } else {
                        pane_action
                    }
                }
            }
            EventType::Resize(width, height) => {
                // Handle terminal resize - update cached dimensions
                self.sidebar_width = self.calculate_sidebar_width(width);
                self.screen_width = width;
                self.screen_height = height;
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

impl AppComponent {
    /// Calculate sidebar width based on configured columns
    fn calculate_sidebar_width(&self, screen_width: u16) -> u16 {
        let sidebar_columns = self.sidebar_width_override.unwrap_or_else(|| self.sidebar.preferred_width());
        let max_sidebar_width = screen_width.saturating_sub(MAIN_AREA_MIN_WIDTH);
        sidebar_columns.min(max_sidebar_width)
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
        let page_chunks = if self.config.ui.shortcut_bar_visible {
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(rect)
        } else {
            Layout::vertical([Constraint::Min(0), Constraint::Length(0)]).split(rect)
        };
        let content_rect = page_chunks[0];

        // Create layout: sidebar (configurable width) | task list (remainder)
        let sidebar_width = if self.sidebar_visible {
            self.calculate_sidebar_width(content_rect.width)
        } else {
            0
        };

        // Update cached dimensions for mouse event handling
        self.sidebar_width = sidebar_width;
        self.screen_width = rect.width;
        self.screen_height = rect.height;

        let main_chunks =
            Layout::horizontal([Constraint::Length(sidebar_width), Constraint::Min(0)]).split(content_rect);

        // Render components
        if self.sidebar_visible {
            self.sidebar.set_focused(self.active_pane == ActivePane::Navigation);
            self.sidebar.render(f, main_chunks[0]);
        }
        self.task_list.set_focused(self.active_pane == ActivePane::Tasks);
        self.task_list.set_processing(self.task_manager.processing_description());
        self.task_list.render(f, main_chunks[1]);

        if self.config.ui.shortcut_bar_visible {
            Self::render_shortcut_bar(f, page_chunks[1], &self.state.sidebar_selection);
        }

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
    fn shortcut_bar_items(selection: &SidebarSelection) -> &'static [(&'static str, &'static str)] {
        if selection == &SidebarSelection::Trash {
            &[
                ("j/k", "navigate"),
                ("x", "select"),
                ("d", "restore"),
                ("D", "empty trash"),
                ("/", "search"),
                ("r", "sync"),
                ("?", "help"),
                ("q", "quit"),
            ]
        } else if selection == &SidebarSelection::Agenda {
            &[
                ("j/k", "navigate"),
                ("Space", "toggle complete"),
                ("s", "set time"),
                ("/", "search"),
                ("r", "sync"),
                ("?", "help"),
                ("q", "quit"),
            ]
        } else {
            &[
                ("j/k", "navigate"),
                ("x", "select"),
                ("Space", "toggle complete"),
                ("a", "add"),
                ("t", "today"),
                ("/", "search"),
                ("r", "sync"),
                ("?", "help"),
                ("q", "quit"),
            ]
        }
    }

    fn render_shortcut_bar(f: &mut Frame, rect: Rect, selection: &SidebarSelection) {
        use ratatui::{
            style::{Color, Style},
            text::{Line, Span},
            widgets::Paragraph,
        };

        let shortcuts = Self::shortcut_bar_items(selection);
        let mut spans = Vec::new();
        for (index, (key, label)) in shortcuts.iter().enumerate() {
            if index > 0 {
                spans.push(Span::raw("  "));
            }
            spans.push(Span::styled(*key, Style::default().fg(Color::Cyan)));
            spans.push(Span::styled(
                format!(" {}", label),
                Style::default().fg(Color::DarkGray),
            ));
        }
        f.render_widget(Paragraph::new(Line::from(spans)), rect);
    }

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
            UI_LOADING_DATA
        } else {
            UI_SYNCING_WITH_TODOIST
        };

        let spinner = "⟳";
        let content = Paragraph::new(Line::from(Span::styled(
            format!("{} {}…", spinner, title),
            Style::default().fg(Color::Yellow),
        )))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).style(Style::default().fg(Color::Yellow)));

        f.render_widget(Clear, popup_area);
        f.render_widget(content, popup_area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{backend, project};
    use crate::storage::LocalStorage;
    use sea_orm::{EntityTrait, Set};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    async fn test_app() -> (AppComponent, Arc<Mutex<LocalStorage>>, std::path::PathBuf) {
        let db_path = std::env::temp_dir().join(format!("terminalist-snapshot-{}.db", Uuid::new_v4()));
        let storage = Arc::new(Mutex::new(LocalStorage::new_at(db_path.clone()).await.unwrap()));
        let sync_service = SyncService::new_for_test(storage.clone(), Uuid::new_v4());
        (AppComponent::new(sync_service, Config::default()), storage, db_path)
    }

    fn snapshot(generation: u64, selection: SidebarSelection, projects: Vec<project::Model>) -> ViewSnapshot {
        ViewSnapshot {
            generation,
            selection,
            is_initial: false,
            projects,
            labels: Vec::new(),
            sections: Vec::new(),
            tasks: Vec::new(),
            all_tasks: Vec::new(),
            navigation_counts: NavigationCounts::default(),
        }
    }

    fn project_named(name: &str) -> project::Model {
        project::Model {
            uuid: Uuid::new_v4(),
            backend_uuid: Uuid::new_v4(),
            remote_id: name.to_string(),
            name: name.to_string(),
            is_favorite: false,
            is_inbox_project: false,
            order_index: 0,
            parent_uuid: None,
        }
    }

    #[test]
    fn only_single_task_completion_uses_the_non_blocking_path() {
        let pending_task = Uuid::new_v4();
        let other_task = Uuid::new_v4();

        assert_eq!(
            AppComponent::single_task_completion(&[(pending_task, false)]),
            Some(pending_task)
        );
        assert_eq!(AppComponent::single_task_completion(&[(pending_task, true)]), None);
        assert_eq!(
            AppComponent::single_task_completion(&[(pending_task, false), (other_task, false)]),
            None
        );
    }

    #[test]
    fn trash_shortcut_bar_replaces_task_actions_with_restore_actions() {
        let shortcuts = AppComponent::shortcut_bar_items(&SidebarSelection::Trash);

        assert!(shortcuts.contains(&("d", "restore")));
        assert!(shortcuts.contains(&("D", "empty trash")));
        assert!(!shortcuts
            .iter()
            .any(|(_, label)| ["toggle complete", "add", "today"].contains(label)));
    }

    #[tokio::test]
    async fn stale_view_snapshot_cannot_replace_the_latest_navigation_result() {
        let (mut app, storage, db_path) = test_app().await;
        app.latest_requested_generation = 2;
        app.state.sidebar_selection = SidebarSelection::Tomorrow;

        app.handle_app_action(Action::DataLoaded(Box::new(snapshot(
            2,
            SidebarSelection::Tomorrow,
            vec![project_named("Latest")],
        ))))
        .await;
        app.handle_app_action(Action::DataLoaded(Box::new(snapshot(
            1,
            SidebarSelection::Today,
            vec![project_named("Stale")],
        ))))
        .await;

        assert_eq!(app.latest_applied_generation, 2);
        assert_eq!(app.state.projects[0].name, "Latest");
        storage.lock().await.conn.clone().close().await.unwrap();
        std::fs::remove_file(db_path).unwrap();
    }

    #[tokio::test]
    async fn current_load_failure_preserves_the_last_accepted_snapshot() {
        let (mut app, storage, db_path) = test_app().await;
        app.latest_requested_generation = 3;
        app.state.sidebar_selection = SidebarSelection::Upcoming;
        app.state.projects = vec![project_named("Still visible")];

        let follow_up = app
            .handle_app_action(Action::DataLoadFailed {
                generation: 3,
                selection: SidebarSelection::Upcoming,
                message: "offline".to_string(),
            })
            .await;

        assert!(matches!(follow_up, Action::ShowDialog(DialogType::Error(_))));
        assert_eq!(app.state.projects[0].name, "Still visible");
        assert_eq!(app.state.error_message.as_deref(), Some("offline"));
        storage.lock().await.conn.clone().close().await.unwrap();
        std::fs::remove_file(db_path).unwrap();
    }

    #[tokio::test]
    async fn startup_loads_cached_data_when_the_backend_is_unavailable() {
        let db_path = std::env::temp_dir().join(format!("terminalist-offline-{}.db", Uuid::new_v4()));
        let storage = LocalStorage::new_at(db_path.clone()).await.unwrap();
        let backend_uuid = Uuid::new_v4();

        backend::Entity::insert(backend::ActiveModel {
            uuid: Set(backend_uuid),
            backend_type: Set("test".to_string()),
            name: Set("Unavailable backend".to_string()),
            is_enabled: Set(true),
            credentials: Set("{}".to_string()),
            settings: Set("{}".to_string()),
        })
        .exec(&storage.conn)
        .await
        .unwrap();
        project::Entity::insert(project::ActiveModel {
            uuid: Set(Uuid::new_v4()),
            backend_uuid: Set(backend_uuid),
            remote_id: Set("cached-project".to_string()),
            name: Set("Cached project".to_string()),
            is_favorite: Set(false),
            is_inbox_project: Set(false),
            order_index: Set(1),
            parent_uuid: Set(None),
        })
        .exec(&storage.conn)
        .await
        .unwrap();

        let storage = Arc::new(Mutex::new(storage));
        let sync_service = SyncService::new_for_test(storage.clone(), backend_uuid);
        let mut app = AppComponent::new(sync_service, Config::default());
        app.trigger_initial_sync();

        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            loop {
                let actions = app.process_background_actions();
                for action in actions {
                    app.handle_app_action(action).await;
                }
                if app.total_projects() == 1 && app.state.error_message.is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            }
        })
        .await
        .expect("cached data and the backend error should both load");

        assert_eq!(app.total_projects(), 1);
        assert_eq!(app.state.projects[0].name, "Cached project");
        assert!(app.state.error_message.is_some());

        app.task_manager.cancel_all_tasks();
        storage.lock().await.conn.clone().close().await.unwrap();
        std::fs::remove_file(db_path).unwrap();
    }

    #[tokio::test]
    async fn task_operation_refreshes_an_open_search_query() {
        let (mut app, storage, db_path) = test_app().await;
        app.dialog.dialog_type = Some(DialogType::TaskSearch);
        app.dialog.input_buffer = "needle".to_string();

        app.handle_app_action(Action::RefreshData).await;

        assert_eq!(app.task_manager.task_count(), 2);
        app.task_manager.cancel_all_tasks();
        storage.lock().await.conn.clone().close().await.unwrap();
        std::fs::remove_file(db_path).unwrap();
    }
}
