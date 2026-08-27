use crate::config::Config;
use crate::constants::*;
use crate::entities::{label, project, section, task};
use crate::sync::{SyncService, SyncStatus};
use crate::theme::{self, ThemeWarning};
use crate::ui::components::sync_toast::sync_completed_successfully;
use crate::ui::components::{should_auto_sync, DialogComponent, SidebarComponent, SyncToast, TaskListComponent};
use crate::ui::core::SidebarSelection;
use crate::ui::core::{
    actions::{Action, DialogType, SelectionPolicy},
    event_handler::EventType,
    task_manager::{TaskId, TaskManager},
    Component,
};
use crate::utils::datetime;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use log::{error, info};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    Frame,
};
use std::time::Instant;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Application state separate from UI concerns
#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub projects: Vec<project::Model>,
    pub tasks: Vec<task::Model>,
    pub labels: Vec<label::Model>,
    pub sections: Vec<section::Model>,
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
        projects: Vec<project::Model>,
        labels: Vec<label::Model>,
        sections: Vec<section::Model>,
        tasks: Vec<task::Model>,
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
    sync_toast: SyncToast,

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
    /// True from startup until the first local data load lands and establishes the sidebar
    /// selection from `config.ui.default_project`. Nothing else may set the initial selection:
    /// doing it a second time snaps the user back to the default view and rebuilds the task
    /// list underneath them.
    initial_selection_pending: bool,
    last_sync_attempt_at: Option<Instant>,

    // Layout state
    sidebar_visible: bool,
    sidebar_width: u16,
    screen_width: u16,
    screen_height: u16,
}

impl AppComponent {
    pub fn new(sync_service: SyncService, config: Config, theme_warnings: Vec<ThemeWarning>) -> Self {
        let sidebar = SidebarComponent::new();
        let task_list = TaskListComponent::new();
        let (task_manager, background_action_rx) = TaskManager::new();

        let state = AppState {
            loading: true,
            ..Default::default()
        };

        let mut dialog = DialogComponent::new();
        if let Some(message) = theme::format_warnings(&theme_warnings) {
            dialog.update(Action::ShowDialog(DialogType::Info(message)));
        }

        Self {
            sidebar,
            task_list,
            dialog,
            sync_toast: SyncToast::new(),
            state,
            sync_service,
            task_manager,
            background_action_rx,
            sidebar_visible: config.ui.sidebar_visible,
            config,
            should_quit: false,
            active_sync_task: None,
            initial_selection_pending: false,
            last_sync_attempt_at: None,
            sidebar_width: 30, // Default width
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

    /// Whether a tick can change what the sync toast shows — true only while a success
    /// toast is counting down to its own expiry.
    ///
    /// The render loop uses this, not "is the toast visible", to decide whether a tick has
    /// to repaint. A failed sync's toast stays up until the user dismisses it, so keying the
    /// repaint off visibility would redraw the whole TUI ten times a second for as long as
    /// that notice is on screen.
    pub fn sync_toast_expires_on_tick(&self) -> bool {
        self.sync_toast.expires_on_tick()
    }

    /// The sidebar entry the task list is currently showing.
    pub fn sidebar_selection(&self) -> &SidebarSelection {
        &self.state.sidebar_selection
    }

    /// Await the next background action rather than draining whatever has already arrived.
    ///
    /// Same channel as [`Self::process_background_actions`], which the render loop polls
    /// without blocking. Widened for the integration tests under `tests/ui/`, which need to
    /// step the component across a background data load deterministically — no polling, no
    /// sleeping, no wall-clock dependency.
    pub async fn next_background_action(&mut self) -> Option<Action> {
        self.background_action_rx.recv().await
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
    ///
    /// The initial *selection* is established by the local data load scheduled here, not by the
    /// sync finishing. `initial_selection_pending` stays set until that load is handled, so a
    /// sync that fails, succeeds, or never happens at all (debug mode) all end up in the same
    /// place.
    pub fn trigger_initial_sync(&mut self) {
        if self.active_sync_task.is_some() {
            return;
        }

        // Paint cached data immediately so the user has something to look at and navigate; the
        // background sync refreshes the view again once it completes. Debug mode differs by
        // exactly one thing — it skips the network sync — so the pending-selection lifecycle is
        // identical on both paths and cannot drift apart.
        self.initial_selection_pending = true;
        self.schedule_initial_data_fetch();

        if self.sync_service.is_debug_mode() {
            info!("AppComponent: Skipping initial sync (debug mode), loading cached data only");
        } else {
            info!("AppComponent: Starting initial sync");
            self.start_background_sync();
            info!("AppComponent: Initial sync scheduled");
        }
    }

    /// Set initial sidebar selection based on config
    fn set_initial_sidebar_selection(&mut self) {
        let selection = match self.config.ui.default_project.as_str() {
            "inbox" => {
                // Find inbox project
                if let Some(inbox_index) = self.state.projects.iter().position(|p| p.is_inbox_project) {
                    SidebarSelection::Project(inbox_index)
                } else {
                    SidebarSelection::Today
                }
            }
            "today" => SidebarSelection::Today,
            "tomorrow" => SidebarSelection::Tomorrow,
            "upcoming" => SidebarSelection::Upcoming,
            project_id_or_name => {
                // Try to find project by ID first (parse as UUID), then by name
                if let Ok(uuid) = Uuid::parse_str(project_id_or_name) {
                    if let Some(project_index) = self.state.projects.iter().position(|p| p.uuid == uuid) {
                        SidebarSelection::Project(project_index)
                    } else if let Some(project_index) =
                        self.state.projects.iter().position(|p| p.name == project_id_or_name)
                    {
                        SidebarSelection::Project(project_index)
                    } else {
                        SidebarSelection::Today
                    }
                } else if let Some(project_index) =
                    self.state.projects.iter().position(|p| p.name == project_id_or_name)
                {
                    SidebarSelection::Project(project_index)
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

    /// Update all components with current data.
    ///
    /// `selection_policy` is forwarded to the task list's reload and only matters when the
    /// data actually changed (a `DataLoaded`/`InitialDataLoaded` handler). Callers that are not
    /// reacting to a fresh data load pass `SelectionPolicy::KeepIndex`: the data has not
    /// changed, so it is a no-op, and it is the conservative choice.
    fn sync_component_data(&mut self, selection_policy: SelectionPolicy) {
        // Update sidebar
        self.sidebar.update_data(self.state.projects.clone(), self.state.labels.clone());
        self.sidebar.selection = self.state.sidebar_selection.clone();
        self.sidebar.update_theme(self.config.theme.clone());

        // Update task list
        self.task_list.update_display_config(self.config.display.clone());
        self.task_list.update_theme(self.config.theme.clone());
        self.task_list.update_data(
            self.state.tasks.clone(),
            self.state.sections.clone(),
            self.state.projects.clone(),
            self.state.labels.clone(),
            self.state.sidebar_selection.clone(),
            selection_policy,
        );

        // Update dialog
        self.dialog.update_display_config(self.config.display.clone());
        self.dialog.update_theme(self.config.theme.clone());
        self.dialog.update_data_with_tasks(
            self.state.projects.clone(),
            self.state.labels.clone(),
            self.state.tasks.clone(),
        );
        self.dialog.set_sync_service(self.sync_service.clone());

        // Update sync toast
        self.sync_toast.update_theme(self.config.theme.clone());
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
                    SidebarSelection::Project(index) => {
                        if let Some(project) = self.state.projects.get(*index) {
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
                    SidebarSelection::Tomorrow => {
                        info!("Global key: 'D' - cannot delete Tomorrow view");
                        Action::ShowDialog(DialogType::Info("Cannot delete the Tomorrow view".to_string()))
                    }
                    SidebarSelection::Upcoming => {
                        info!("Global key: 'D' - cannot delete Upcoming view");
                        Action::ShowDialog(DialogType::Info("Cannot delete the Upcoming view".to_string()))
                    }
                    SidebarSelection::Label(index) => {
                        if let Some(label) = self.state.labels.get(*index) {
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
                    SidebarSelection::Project(index) => {
                        if let Some(project) = self.state.projects.get(*index) {
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
                    SidebarSelection::Tomorrow => {
                        info!("Global key: 'E' - cannot edit Tomorrow view");
                        Action::ShowDialog(DialogType::Info("Cannot edit the Tomorrow view".to_string()))
                    }
                    SidebarSelection::Upcoming => {
                        info!("Global key: 'E' - cannot edit Upcoming view");
                        Action::ShowDialog(DialogType::Info("Cannot edit the Upcoming view".to_string()))
                    }
                    SidebarSelection::Label(index) => {
                        if let Some(label) = self.state.labels.get(*index) {
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
                // Show the "syncing" toast regardless of whether this call actually
                // starts a new sync: it also covers the async "sync started"
                // notification that arrives after `start_background_sync` was already
                // called directly (e.g. from `trigger_initial_sync`), which otherwise
                // would hit the "already in progress" branch below and never surface.
                self.sync_toast.started();
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
                // Schedule a data fetch directly from local storage without API sync. User-
                // initiated (the debug-mode `R` key), so the cursor stays on its row.
                self.schedule_data_fetch(SelectionPolicy::KeepIndex);
                Action::None
            }
            Action::SyncCompleted(status) => {
                info!("Sync: Completed with status {:?}", status);
                self.active_sync_task = None;
                self.state.loading = false;
                let now = Instant::now();
                // Record every terminal attempt, success or failure, so a failure waits
                // a full auto-sync interval before retrying instead of re-firing on the
                // very next tick (see `should_auto_sync`'s doc comment).
                self.last_sync_attempt_at = Some(now);

                if sync_completed_successfully(&status) {
                    self.sync_toast.succeeded(now);
                } else {
                    error!("Sync: Completed with a non-success status: {:?}", status);
                    self.sync_toast.failed();
                }

                // Extract data from sync status and update components. The actual data reload
                // (if any) happens asynchronously via `update_data_from_sync`'s `DataLoaded`,
                // which carries its own policy; this call just repaints components with
                // whatever data is already in `self.state`, unchanged, so KeepIndex is a no-op.
                self.update_data_from_sync(status);
                self.sync_component_data(SelectionPolicy::KeepIndex);

                Action::None
            }
            Action::SyncFailed(error) => {
                error!("Sync: Failed with error: {}", error);
                self.active_sync_task = None;
                self.state.loading = false;
                // `initial_selection_pending` is deliberately NOT cleared here. It is owned by
                // the local data load, which runs independently of the sync and still has to
                // establish the initial selection; clearing it on a sync failure would leave a
                // startup with no network stuck on whatever `SidebarSelection::default()` is,
                // ignoring `default_project`.

                // Record the attempt, same reasoning as the `SyncCompleted` arm above.
                self.last_sync_attempt_at = Some(Instant::now());
                self.sync_toast.failed();
                Action::None
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

                info!("Navigation: Sidebar selection changed to {}", selection_desc);
                self.state.sidebar_selection = selection.clone();
                // Reload data for the new selection. User-initiated navigation, so KeepIndex.
                self.schedule_data_fetch(SelectionPolicy::KeepIndex);
                info!("Navigation: Scheduled data fetch for new selection");
                Action::None
            }
            // Task operations with background execution
            Action::CreateTask { content, project_uuid } => {
                let project_desc = match &project_uuid {
                    Some(uuid) => format!(" in project {}", uuid),
                    None => " in inbox".to_string(),
                };
                info!("Task: Creating task with content '{}'{}", content, project_desc);

                // Format task info to include both content and project_uuid
                let task_info = match project_uuid {
                    Some(pid) => format!("{}|{}", content, pid),
                    None => content,
                };
                self.spawn_task_operation("Create task".to_string(), task_info);
                Action::None
            }
            Action::CompleteTask(task_id) => {
                // Find the task being completed
                let sync_service = self.sync_service.clone();
                if let Ok(task_uuid) = Uuid::parse_str(&task_id) {
                    if let Ok(Some(task)) = sync_service.get_task_by_id(&task_uuid).await {
                        let task_desc = format!("ID {} '{}'", task_id, task.content);

                        info!("Task: Completing task {}", task_desc);

                        // Todoist API automatically handles subtasks when parent is completed
                        self.spawn_task_operation("Complete task".to_string(), task_id);
                    } else {
                        info!("Task: Cannot complete - task {} not found", task_id);
                    }
                } else {
                    info!("Task: Cannot complete - invalid UUID {}", task_id);
                }
                Action::None
            }
            Action::CyclePriority(task_id) => {
                // Find task and cycle its priority
                let sync_service = self.sync_service.clone();
                if let Ok(task_uuid) = Uuid::parse_str(&task_id) {
                    if let Ok(Some(task)) = sync_service.get_task_by_id(&task_uuid).await {
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
                        self.spawn_task_operation(
                            "Cycle priority".to_string(),
                            format!("{}|{}", task_id, new_priority),
                        );
                    } else {
                        info!("Task: Cannot cycle priority - task {} not found", task_id);
                    }
                } else {
                    info!("Task: Cannot cycle priority - invalid UUID {}", task_id);
                }
                Action::None
            }
            Action::DeleteTask(task_id) => {
                // Find task name for better logging
                let sync_service = self.sync_service.clone();
                let task_desc = if let Ok(task_uuid) = Uuid::parse_str(&task_id) {
                    if let Ok(Some(task)) = sync_service.get_task_by_id(&task_uuid).await {
                        format!("ID {} '{}'", task_id, task.content)
                    } else {
                        format!("ID {} [unknown]", task_id)
                    }
                } else {
                    format!("ID {} [invalid UUID]", task_id)
                };
                info!("Task: Deleting task {}", task_desc);
                self.spawn_task_operation("Delete task".to_string(), task_id);
                Action::None
            }
            Action::SetTaskDueToday(task_id) => {
                // Find task name for better logging
                let sync_service = self.sync_service.clone();
                let task_id_str = task_id.to_string();
                let task_desc = if let Ok(Some(task)) = sync_service.get_task_by_id(&task_id).await {
                    format!("ID {} '{}'", task_id, task.content)
                } else {
                    format!("ID {} [unknown]", task_id)
                };
                info!("Task: Setting due date to today for task {}", task_desc);
                self.spawn_task_operation("Set task due today".to_string(), format!("{}|today", task_id_str));
                Action::None
            }
            Action::SetTaskDueTomorrow(task_id) => {
                // Find task name for better logging
                let sync_service = self.sync_service.clone();
                let task_id_str = task_id.to_string();
                let task_desc = if let Ok(Some(task)) = sync_service.get_task_by_id(&task_id).await {
                    format!("ID {} '{}'", task_id, task.content)
                } else {
                    format!("ID {} [unknown]", task_id)
                };
                info!("Task: Setting due date to tomorrow for task {}", task_desc);
                self.spawn_task_operation("Set task due tomorrow".to_string(), format!("{}|tomorrow", task_id_str));
                Action::None
            }
            Action::SetTaskDueNextWeek(task_id) => {
                // Find task name for better logging
                let sync_service = self.sync_service.clone();
                let task_id_str = task_id.to_string();
                let task_desc = if let Ok(Some(task)) = sync_service.get_task_by_id(&task_id).await {
                    format!("ID {} '{}'", task_id, task.content)
                } else {
                    format!("ID {} [unknown]", task_id)
                };
                info!("Task: Setting due date to next week for task {}", task_desc);
                self.spawn_task_operation(
                    "Set task due next week".to_string(),
                    format!("{}|next_week", task_id_str),
                );
                Action::None
            }
            Action::SetTaskDueWeekEnd(task_id) => {
                // Find task name for better logging
                let sync_service = self.sync_service.clone();
                let task_id_str = task_id.to_string();
                let task_desc = if let Ok(Some(task)) = sync_service.get_task_by_id(&task_id).await {
                    format!("ID {} '{}'", task_id, task.content)
                } else {
                    format!("ID {} [unknown]", task_id)
                };
                info!("Task: Setting due date to weekend for task {}", task_desc);
                self.spawn_task_operation("Set task due weekend".to_string(), format!("{}|weekend", task_id_str));
                Action::None
            }
            Action::EditTask { task_uuid, content } => {
                info!("Task: Editing task UUID {} with new content '{}'", task_uuid, content);
                self.spawn_task_operation("Edit task".to_string(), format!("{}: {}", task_uuid, content));
                Action::None
            }
            Action::RestoreTask(task_id) => {
                info!("Task: Restoring task {}", task_id);
                self.spawn_task_operation("Restore task".to_string(), task_id);
                Action::None
            }
            Action::CreateProject { name, parent_uuid } => {
                let parent_desc = match &parent_uuid {
                    Some(uuid) => format!(" with parent {}", uuid),
                    None => "".to_string(),
                };
                info!("Project: Creating project '{}'{}", name, parent_desc);

                // Format project info to include both name and parent_uuid
                let project_info = match parent_uuid {
                    Some(pid) => format!("{}|{}", name, pid),
                    None => name,
                };
                self.spawn_task_operation("Create project".to_string(), project_info);
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
                self.spawn_task_operation("Delete project".to_string(), project_id.to_string());
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
                self.spawn_task_operation("Delete label".to_string(), label_id.to_string());
                Action::None
            }
            Action::CreateLabel { name } => {
                info!("Label: Creating label '{}'", name);
                self.spawn_task_operation("Create label".to_string(), name);
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
                self.spawn_task_operation("Edit project".to_string(), format!("{}: {}", project_uuid, name));
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
                self.spawn_task_operation("Edit label".to_string(), format!("{}: {}", label_uuid, name));
                Action::None
            }
            Action::InitialDataLoaded {
                projects,
                labels,
                sections,
                tasks,
            } => {
                info!(
                    "InitialData: Loaded {} projects, {} labels, {} sections, {} tasks",
                    projects.len(),
                    labels.len(),
                    sections.len(),
                    tasks.len()
                );

                // Update app state with loaded data
                self.state.update_data(projects, labels, sections, tasks);

                // The initial selection is established exactly once, here, on the first local
                // load after startup. Guarding on the flag (and clearing it immediately) is what
                // keeps a later reload from snapping the user back to `default_project` and
                // rebuilding the task list under them.
                if self.initial_selection_pending {
                    self.initial_selection_pending = false;

                    // Set initial sidebar selection based on config (now we have projects loaded)
                    self.set_initial_sidebar_selection();
                    info!("AppComponent: Set initial sidebar selection after initial data load");

                    // Fetch data for the newly selected sidebar item. User-initiated in effect
                    // (it applies `default_project`, not a task the user was looking at), so
                    // KeepIndex.
                    self.schedule_data_fetch(SelectionPolicy::KeepIndex);
                    info!("AppComponent: Scheduled data fetch for initial sidebar selection");
                }

                // No prior selection exists yet at startup, so there is nothing to keep or
                // follow: KeepIndex is a no-op here, same as for every other non-data-load
                // caller of `sync_component_data`.
                self.sync_component_data(SelectionPolicy::KeepIndex);
                info!("InitialData: Updated all component data after initial data load");
                Action::None
            }
            Action::DataLoaded {
                projects,
                labels,
                sections,
                tasks,
                selection_policy,
            } => {
                info!(
                    "Data: Loaded {} projects, {} labels, {} sections, {} tasks",
                    projects.len(),
                    labels.len(),
                    sections.len(),
                    tasks.len()
                );

                // Update app state with loaded data
                self.state.update_data(projects, labels, sections, tasks);
                self.sync_component_data(selection_policy);
                info!("Data: Updated all component data after data load");
                Action::None
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
                // Schedule a data fetch to reload current view with updated data. This is
                // always user-initiated (a task operation the user just performed), so the
                // cursor stays on its row rather than following the task it just changed --
                // e.g. pressing `t` to mark an overdue task due "today" must not drag the
                // cursor along with it as it moves out of the Overdue section.
                self.schedule_data_fetch(SelectionPolicy::KeepIndex);
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

    /// Spawn a generic task operation (now with actual API calls and data refresh)
    fn spawn_task_operation(&mut self, operation_name: String, task_info: String) {
        let description = format!("{}: {}", operation_name, task_info);
        let op_name = operation_name.clone();
        let sync_service = self.sync_service.clone();
        info!("Background: Spawning task operation '{}'", description);

        let _task_id = self.task_manager.spawn_task_operation(
            move || async move {
                let result = match op_name.as_str() {
                    "Complete task" => match Uuid::parse_str(&task_info) {
                        Ok(task_uuid) => match sync_service.complete_task(&task_uuid).await {
                            Ok(()) => Ok(format!("{}: {}", SUCCESS_TASK_COMPLETED, task_info)),
                            Err(e) => Err(format!("{}: {}", ERROR_TASK_COMPLETION_FAILED, e)),
                        },
                        Err(e) => Err(format!("Invalid task UUID: {}", e)),
                    },
                    "Delete task" => match Uuid::parse_str(&task_info) {
                        Ok(task_uuid) => match sync_service.delete_task(&task_uuid).await {
                            Ok(()) => Ok(format!("{}: {}", SUCCESS_TASK_DELETED, task_info)),
                            Err(e) => Err(format!("{}: {}", ERROR_TASK_DELETE_FAILED, e)),
                        },
                        Err(e) => Err(format!("Invalid task UUID: {}", e)),
                    },
                    "Cycle priority" => {
                        // task_info format: "task_id|new_priority"
                        if let Some((task_id_str, priority_str)) = task_info.split_once('|') {
                            match Uuid::parse_str(task_id_str) {
                                Ok(task_uuid) => {
                                    if let Ok(priority) = priority_str.parse::<i32>() {
                                        match sync_service.update_task_priority(&task_uuid, priority).await {
                                            Ok(()) => Ok(format!(
                                                "{}{}: {}",
                                                SUCCESS_TASK_PRIORITY_UPDATED, priority, task_id_str
                                            )),
                                            Err(e) => Err(format!("{}: {}", ERROR_TASK_PRIORITY_FAILED, e)),
                                        }
                                    } else {
                                        Err(ERROR_INVALID_PRIORITY_FORMAT.to_string())
                                    }
                                }
                                Err(e) => Err(format!("Invalid task UUID: {}", e)),
                            }
                        } else {
                            Err(ERROR_INVALID_PRIORITY_INFO.to_string())
                        }
                    }
                    "Set task due today" => {
                        // task_info format: "task_id|today"
                        if let Some((task_id_str, _)) = task_info.split_once('|') {
                            match Uuid::parse_str(task_id_str) {
                                Ok(task_uuid) => {
                                    let today = datetime::format_today();
                                    match sync_service.update_task_due_date(&task_uuid, Some(&today)).await {
                                        Ok(()) => Ok(format!("{}: {}", SUCCESS_TASK_DUE_TODAY, task_id_str)),
                                        Err(e) => Err(format!("{}: {}", ERROR_TASK_DUE_DATE_FAILED, e)),
                                    }
                                }
                                Err(e) => Err(format!("Invalid task UUID: {}", e)),
                            }
                        } else {
                            Err(ERROR_INVALID_DATE_FORMAT.to_string())
                        }
                    }
                    "Set task due tomorrow" => {
                        // task_info format: "task_id|tomorrow"
                        if let Some((task_id_str, _)) = task_info.split_once('|') {
                            match Uuid::parse_str(task_id_str) {
                                Ok(task_uuid) => {
                                    let tomorrow = datetime::format_date_with_offset(1);
                                    match sync_service.update_task_due_date(&task_uuid, Some(&tomorrow)).await {
                                        Ok(()) => Ok(format!("{}: {}", SUCCESS_TASK_DUE_TOMORROW, task_id_str)),
                                        Err(e) => Err(format!("{}: {}", ERROR_TASK_DUE_DATE_FAILED, e)),
                                    }
                                }
                                Err(e) => Err(format!("Invalid task UUID: {}", e)),
                            }
                        } else {
                            Err(ERROR_INVALID_DATE_FORMAT.to_string())
                        }
                    }
                    "Set task due next week" => {
                        // task_info format: "task_id|next_week"
                        if let Some((task_id_str, _)) = task_info.split_once('|') {
                            match Uuid::parse_str(task_id_str) {
                                Ok(task_uuid) => {
                                    let today = chrono::Local::now().date_naive();
                                    let next_monday = crate::utils::datetime::next_weekday(today, chrono::Weekday::Mon);
                                    let next_monday_str = crate::utils::datetime::format_ymd(next_monday);
                                    match sync_service.update_task_due_date(&task_uuid, Some(&next_monday_str)).await {
                                        Ok(()) => Ok(format!("{}: {}", SUCCESS_TASK_DUE_MONDAY, task_id_str)),
                                        Err(e) => Err(format!("{}: {}", ERROR_TASK_DUE_DATE_FAILED, e)),
                                    }
                                }
                                Err(e) => Err(format!("Invalid task UUID: {}", e)),
                            }
                        } else {
                            Err(ERROR_INVALID_DATE_FORMAT.to_string())
                        }
                    }
                    "Set task due weekend" => {
                        // task_info format: "task_id|weekend"
                        if let Some((task_id_str, _)) = task_info.split_once('|') {
                            match Uuid::parse_str(task_id_str) {
                                Ok(task_uuid) => {
                                    let today = chrono::Local::now().date_naive();
                                    let next_saturday =
                                        crate::utils::datetime::next_weekday(today, chrono::Weekday::Sat);
                                    let next_saturday_str = crate::utils::datetime::format_ymd(next_saturday);
                                    match sync_service.update_task_due_date(&task_uuid, Some(&next_saturday_str)).await
                                    {
                                        Ok(()) => Ok(format!("{}: {}", SUCCESS_TASK_DUE_SATURDAY, task_id_str)),
                                        Err(e) => Err(format!("{}: {}", ERROR_TASK_DUE_DATE_FAILED, e)),
                                    }
                                }
                                Err(e) => Err(format!("Invalid task UUID: {}", e)),
                            }
                        } else {
                            Err(ERROR_INVALID_DATE_FORMAT.to_string())
                        }
                    }
                    "Create task" => {
                        // task_info format: "content|project_id" or just "content" for inbox
                        if let Some((content, project_id_str)) = task_info.split_once('|') {
                            // Task has a specific project - parse the UUID
                            match Uuid::parse_str(project_id_str) {
                                Ok(project_uuid) => match sync_service.create_task(content, Some(project_uuid)).await {
                                    Ok(()) => Ok(format!("{}: {}", SUCCESS_TASK_CREATED_PROJECT, content)),
                                    Err(e) => Err(format!("{}: {}", ERROR_TASK_CREATE_FAILED, e)),
                                },
                                Err(e) => Err(format!("Invalid project UUID: {}", e)),
                            }
                        } else {
                            // Task goes to inbox (no project_id)
                            match sync_service.create_task(&task_info, None).await {
                                Ok(()) => Ok(format!("{}: {}", SUCCESS_TASK_CREATED_INBOX, task_info)),
                                Err(e) => Err(format!("{}: {}", ERROR_TASK_CREATE_FAILED, e)),
                            }
                        }
                    }
                    "Edit task" => {
                        // task_info format: "task_id: new_content"
                        if let Some((task_id_str, content)) = task_info.split_once(": ") {
                            match Uuid::parse_str(task_id_str) {
                                Ok(task_uuid) => match sync_service.update_task_content(&task_uuid, content).await {
                                    Ok(()) => Ok(format!("{}: {}", SUCCESS_TASK_UPDATED, task_id_str)),
                                    Err(e) => Err(format!("{}: {}", ERROR_TASK_UPDATE_FAILED, e)),
                                },
                                Err(e) => Err(format!("Invalid task UUID: {}", e)),
                            }
                        } else {
                            Err(ERROR_INVALID_TASK_EDIT_FORMAT.to_string())
                        }
                    }
                    "Restore task" => match Uuid::parse_str(&task_info) {
                        Ok(task_uuid) => match sync_service.restore_task(&task_uuid).await {
                            Ok(()) => Ok(format!("{}: {}", SUCCESS_TASK_RESTORED, task_info)),
                            Err(e) => Err(format!("{}: {}", ERROR_TASK_RESTORE_FAILED, e)),
                        },
                        Err(e) => Err(format!("Invalid task UUID: {}", e)),
                    },
                    "Create project" => {
                        // project_info format: "name|parent_id" or just "name" for root project
                        if let Some((name, parent_id_str)) = task_info.split_once('|') {
                            // Project has a parent - parse the UUID
                            match Uuid::parse_str(parent_id_str) {
                                Ok(parent_uuid) => match sync_service.create_project(name, Some(parent_uuid)).await {
                                    Ok(()) => Ok(format!("{}: {}", SUCCESS_PROJECT_CREATED_PARENT, name)),
                                    Err(e) => Err(format!("{}: {}", ERROR_PROJECT_CREATE_FAILED, e)),
                                },
                                Err(e) => Err(format!("Invalid parent project UUID: {}", e)),
                            }
                        } else {
                            // Root project (no parent)
                            match sync_service.create_project(&task_info, None).await {
                                Ok(()) => Ok(format!("{}: {}", SUCCESS_PROJECT_CREATED_ROOT, task_info)),
                                Err(e) => Err(format!("{}: {}", ERROR_PROJECT_CREATE_FAILED, e)),
                            }
                        }
                    }
                    "Delete project" => {
                        // task_info is a UUID string
                        match Uuid::parse_str(&task_info) {
                            Ok(project_uuid) => match sync_service.delete_project(&project_uuid).await {
                                Ok(()) => Ok(format!("{}: {}", SUCCESS_PROJECT_DELETED, task_info)),
                                Err(e) => Err(format!("{}: {}", ERROR_PROJECT_DELETE_FAILED, e)),
                            },
                            Err(e) => Err(format!("Invalid project UUID: {}", e)),
                        }
                    }
                    "Delete label" => {
                        // task_info is a UUID string
                        match Uuid::parse_str(&task_info) {
                            Ok(label_uuid) => match sync_service.delete_label(&label_uuid).await {
                                Ok(()) => Ok(format!("{}: {}", SUCCESS_LABEL_DELETED, task_info)),
                                Err(e) => Err(format!("{}: {}", ERROR_LABEL_DELETE_FAILED, e)),
                            },
                            Err(e) => Err(format!("Invalid label UUID: {}", e)),
                        }
                    }
                    "Create label" => match sync_service.create_label(&task_info).await {
                        Ok(()) => Ok(format!("{}: {}", SUCCESS_LABEL_CREATED, task_info)),
                        Err(e) => Err(format!("{}: {}", ERROR_LABEL_CREATE_FAILED, e)),
                    },
                    "Edit project" => {
                        // task_info format: "project_id: new_name"
                        if let Some((project_id_str, name)) = task_info.split_once(": ") {
                            match Uuid::parse_str(project_id_str) {
                                Ok(project_uuid) => {
                                    match sync_service.update_project_content(&project_uuid, name).await {
                                        Ok(()) => Ok(format!("{}: {}", SUCCESS_PROJECT_UPDATED, project_id_str)),
                                        Err(e) => Err(format!("{}: {}", ERROR_PROJECT_UPDATE_FAILED, e)),
                                    }
                                }
                                Err(e) => Err(format!("Invalid project UUID: {}", e)),
                            }
                        } else {
                            Err(ERROR_INVALID_PROJECT_EDIT_FORMAT.to_string())
                        }
                    }
                    "Edit label" => {
                        // task_info format: "label_id: new_name"
                        if let Some((label_id_str, name)) = task_info.split_once(": ") {
                            match Uuid::parse_str(label_id_str) {
                                Ok(label_uuid) => match sync_service.update_label_content(&label_uuid, name).await {
                                    Ok(()) => Ok(format!("{}: {}", SUCCESS_LABEL_UPDATED, label_id_str)),
                                    Err(e) => Err(format!("{}: {}", ERROR_LABEL_UPDATE_FAILED, e)),
                                },
                                Err(e) => Err(format!("Invalid label UUID: {}", e)),
                            }
                        } else {
                            Err(ERROR_INVALID_LABEL_EDIT_FORMAT.to_string())
                        }
                    }
                    _ => Err(format!("{}: {}", ERROR_UNKNOWN_OPERATION, op_name)),
                };

                result.map_err(|e: String| anyhow::anyhow!(e))
            },
            description,
        );
    }

    /// Refresh the view from local storage after a sync landed new data.
    ///
    /// Always the selection-preserving path. The initial sync is not special here: startup
    /// already scheduled its own initial load (see [`Self::trigger_initial_sync`]), so routing
    /// the *completion* through `schedule_initial_data_fetch` too would re-run
    /// `set_initial_sidebar_selection` and undo every bit of navigation the user did while the
    /// sync was running — which is exactly what the non-blocking sync exists to allow. The
    /// auto-sync timer makes that reset recur with no user action at all.
    fn update_data_from_sync(&mut self, status: SyncStatus) {
        // Only proceed if sync was successful. The user may be navigating while the sync
        // lands, so the reload must anchor to the selected task rather than its index.
        if matches!(status, SyncStatus::Success) {
            self.schedule_data_fetch(SelectionPolicy::FollowTask);
        }
    }

    /// Schedule a background task to fetch initial data after sync completion
    fn schedule_initial_data_fetch(&mut self) {
        let _task_id =
            self.task_manager
                .spawn_data_load(self.sync_service.clone(), self.state.sidebar_selection.clone(), None);
    }

    /// Schedule a background task to fetch data after navigation or changes.
    ///
    /// `selection_policy` is forwarded onto the resulting `Action::DataLoaded` unchanged, so
    /// every call site must state its intent explicitly: `KeepIndex` for a user-initiated
    /// reload, `FollowTask` for one triggered by a completed sync. See [`SelectionPolicy`]'s
    /// doc comment for the full rationale.
    fn schedule_data_fetch(&mut self, selection_policy: SelectionPolicy) {
        let _task_id = self.task_manager.spawn_data_load(
            self.sync_service.clone(),
            self.state.sidebar_selection.clone(),
            Some(selection_policy),
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

        // This is the tick path: advance the toast's own timer (so a success toast can
        // expire and a spinner can animate) and decide whether the auto-sync interval
        // has elapsed.
        let now = Instant::now();
        self.sync_toast.tick(now);
        if should_auto_sync(
            self.last_sync_attempt_at,
            now,
            self.config.sync.auto_sync_interval_minutes,
            self.active_sync_task.is_some(),
        ) {
            info!("AppComponent: Auto-sync interval elapsed, starting background sync");
            actions.push(Action::StartSync);
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
                    if self.sidebar_visible && mouse.column < self.sidebar_width {
                        // Mouse is in sidebar area
                        let sidebar_area = Rect::new(0, 0, self.sidebar_width, self.screen_height);
                        self.sidebar.handle_mouse(mouse, sidebar_area)
                    } else {
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
                // Any keypress dismisses a failed sync toast (no-op otherwise).
                self.sync_toast.dismiss();

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

        // Update component data after any changes. Key/mouse/resize events handled here never
        // carry a fresh data load (`DataLoaded`/`InitialDataLoaded` only arrive through the
        // background-action path in the render loop), so this is always a non-data-load
        // caller: KeepIndex.
        self.sync_component_data(SelectionPolicy::KeepIndex);

        Ok(())
    }
}

impl AppComponent {
    /// Calculate sidebar width based on configured columns
    fn calculate_sidebar_width(&self, screen_width: u16) -> u16 {
        let sidebar_columns = self.config.ui.sidebar_width;
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
        // Paint the themed background behind everything (Color::Reset = keep terminal bg).
        f.render_widget(
            ratatui::widgets::Block::default().style(ratatui::style::Style::default().bg(self.config.theme.background)),
            rect,
        );

        // Create layout: sidebar (configurable width) | task list (remainder)
        let sidebar_width = if self.sidebar_visible {
            self.calculate_sidebar_width(rect.width)
        } else {
            0
        };

        // Update cached dimensions for mouse event handling
        self.sidebar_width = sidebar_width;
        self.screen_width = rect.width;
        self.screen_height = rect.height;

        let main_chunks = Layout::horizontal([Constraint::Length(sidebar_width), Constraint::Min(0)]).split(rect);

        // Render components
        if self.sidebar_visible {
            self.sidebar.render(f, main_chunks[0]);
        }
        self.task_list.render(f, main_chunks[1]);

        // Render the non-blocking sync toast in the task list's lower-right corner.
        self.sync_toast.render(f, main_chunks[1]);

        // Render dialog on top if visible (includes help dialog)
        if self.dialog.is_visible() {
            self.dialog.render(f, rect);
        }
    }
}
