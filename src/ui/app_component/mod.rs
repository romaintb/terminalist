pub mod actions;
pub(crate) mod keys;
pub(crate) mod state;

use crate::config::Config;
use crate::constants::*;
use crate::sync::{SyncService, SyncStatus};
use crate::theme::{self, ThemeWarning};
use crate::ui::components::{toast::Toast, DialogComponent, SidebarComponent, TaskListComponent};
use crate::ui::core::{
    actions::{Action, DialogType},
    event_handler::EventType,
    operations::{Due, Operation},
    task_manager::{TaskId, TaskManager},
    Component,
};
use crate::ui::core::{LoadKind, SidebarSelection};
use crossterm::event::KeyEvent;
use log::info;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    Frame,
};
pub use state::AppState;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use uuid::Uuid;

pub struct AppComponent {
    // Component composition
    sidebar: SidebarComponent,
    task_list: TaskListComponent,
    dialog: DialogComponent,

    // Application state
    pub state: AppState,

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
    last_sync_attempt: Option<Instant>,
    toast: Option<Toast>,

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
            state,
            sync_service,
            task_manager,
            background_action_rx,
            sidebar_visible: config.ui.sidebar_visible,
            config,
            should_quit: false,
            active_sync_task: None,
            is_initial_sync: false,
            last_sync_attempt: None,
            toast: None,
            sidebar_width: 30, // Default width
            screen_width: 100, // Default width
            screen_height: 50, // Default height
        }
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// Check if currently syncing
    pub fn is_syncing(&self) -> bool {
        self.active_sync_task.is_some()
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
            self.schedule_data_load(LoadKind::Initial);
            self.is_initial_sync = false;
        } else {
            info!("AppComponent: Loading cached data before initial sync");
            if self.active_sync_task.is_none() {
                self.is_initial_sync = true;
                self.schedule_data_load(LoadKind::Initial);
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

    /// Update all components with current data
    fn sync_component_data(&mut self) {
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
    }

    /// Whether the configured auto-sync interval has elapsed since the last attempt.
    ///
    /// The stamp is taken when a sync *starts*, so a backend that fails fast can't re-fire
    /// on every tick. It stays `None` in debug mode, where no sync ever runs.
    fn auto_sync_due(&self) -> bool {
        let interval = Duration::from_secs(self.config.sync.auto_sync_interval_minutes * 60);
        !interval.is_zero()
            && self.active_sync_task.is_none()
            && self.last_sync_attempt.is_some_and(|at| at.elapsed() >= interval)
    }

    fn start_background_sync(&mut self) {
        self.last_sync_attempt = Some(Instant::now());
        let sync_service = self.sync_service.clone();
        let task_id = self.task_manager.spawn_sync(sync_service);
        self.active_sync_task = Some(task_id);
    }

    /// Hands an operation to the background task manager.
    fn spawn(&mut self, operation: Operation) {
        info!("Background: Spawning task operation '{}'", operation.describe());
        // Deleting the project being viewed would leave the sidebar pointing at nothing.
        let on_success = matches!(operation, Operation::DeleteProject(_))
            .then_some(Action::NavigateToSidebar(SidebarSelection::Today));
        let sync_service = self.sync_service.clone();
        let _task_id = self.task_manager.spawn_task_operation(
            move || async move { operation.run(sync_service).await.map_err(anyhow::Error::msg) },
            on_success,
        );
    }

    fn set_due(&mut self, task: Uuid, when: Due) -> Action {
        info!("Task: Setting due {:?} for {}", when, self.describe_task(task));
        self.spawn(Operation::SetDue { task, when });
        Action::None
    }

    /// Names a task for the log, from the list already in memory. A log line is not
    /// worth a trip to storage; an entry the current view never loaded reads as unknown.
    fn describe_task(&self, uuid: Uuid) -> String {
        match self.state.tasks.iter().find(|task| task.uuid == uuid) {
            Some(task) => format!("ID {} '{}'", uuid, task.content),
            None => format!("ID {} [unknown]", uuid),
        }
    }

    fn describe_project(&self, uuid: Uuid) -> String {
        match self.state.projects.iter().find(|project| project.uuid == uuid) {
            Some(project) => format!("ID {} '{}'", uuid, project.name),
            None => format!("ID {} [unknown]", uuid),
        }
    }

    fn describe_label(&self, uuid: Uuid) -> String {
        match self.state.labels.iter().find(|label| label.uuid == uuid) {
            Some(label) => format!("ID {} '{}'", uuid, label.name),
            None => format!("ID {} [unknown]", uuid),
        }
    }

    fn update_data_from_sync(&mut self, status: SyncStatus) {
        // Only proceed if sync was successful
        if matches!(status, SyncStatus::Success) {
            if self.is_initial_sync {
                // Nothing was on screen to preserve, and the default project is resolvable now.
                self.schedule_data_load(LoadKind::Initial);
                self.is_initial_sync = false;
            } else {
                self.schedule_data_load(LoadKind::Background);
            }
        }
    }

    /// Schedule a background task to reload the current view. `kind` travels with the load
    /// and decides what happens to the cursor when it lands.
    fn schedule_data_load(&mut self, kind: LoadKind) {
        let _task_id =
            self.task_manager
                .spawn_data_load(self.sync_service.clone(), self.state.sidebar_selection.clone(), kind);
    }

    /// Process background actions from task manager
    pub fn process_background_actions(&mut self) -> Vec<Action> {
        let mut actions = Vec::new();

        // Process all available background actions
        while let Ok(action) = self.background_action_rx.try_recv() {
            info!("Background: Received action {:?}", action);
            actions.push(action);
        }

        if self.auto_sync_due() {
            info!("Auto-sync: interval elapsed, queueing sync");
            actions.push(Action::StartSync);
        }

        // Clean up finished tasks
        let completed_tasks = self.task_manager.cleanup_finished_tasks();
        if !completed_tasks.is_empty() {
            let count = completed_tasks.len();
            info!("Background: Cleaned up {} finished tasks", count);
        }

        actions
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

        // Update component data after any changes
        self.sync_component_data();

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

        // Work in flight outranks whatever the last operation left behind. state.loading
        // covers a sync from the moment StartSync is handled; is_syncing() covers the two
        // frames either side of that, before the action lands and after it clears.
        if self.state.loading || self.is_syncing() {
            Toast::spinner(UI_LOADING_DATA, &self.config.theme).render(f, main_chunks[1]);
        } else if let Some(toast) = &self.toast {
            toast.render(f, main_chunks[1]);
        }

        // Render dialog on top if visible (includes help dialog)
        if self.dialog.is_visible() {
            self.dialog.render(f, rect);
        }
    }
}

impl AppComponent {
    /// Whether a notice is parked in the corner.
    pub fn has_toast(&self) -> bool {
        self.toast.is_some()
    }

    /// Drops an expired toast, reporting whether it did. The event loop only repaints on
    /// input or background work, so it sweeps on tick to make stale toasts disappear.
    pub fn sweep_toast(&mut self) -> bool {
        let stale = self.toast.as_ref().is_some_and(Toast::expired);
        if stale {
            self.toast = None;
        }
        stale
    }
}
