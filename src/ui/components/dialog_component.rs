//! Modal dialog component for various user interactions.
//!
//! This component provides a flexible modal dialog system that handles different
//! types of user interactions including task creation/editing, project management,
//! label management, and system functions like search and debugging.

use crate::config::DisplayConfig;
use crate::icons::IconService;
use crate::sync::SyncService;
use crate::todoist::{LabelDisplay, ProjectDisplay, TaskDisplay};
use crate::ui::components::task_list_item_component::{ListItem as TaskListItem, TaskItem};
use crate::ui::core::{
    actions::{Action, DialogType},
    Component,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, widgets::ScrollbarState, Frame};

use crate::ui::components::dialogs::{label_dialogs, project_dialogs, scroll_behavior, system_dialogs, task_dialogs};

/// Modal dialog component that handles various user interactions.
///
/// This component serves as a container for different types of dialogs:
///
/// # Dialog Types
/// - **Task dialogs** - Create, edit, and manage tasks
/// - **Project dialogs** - Create and manage projects
/// - **Label dialogs** - Create and manage labels
/// - **System dialogs** - Search, logs, help, and confirmation dialogs
///
/// # Features
/// - Input handling with cursor management
/// - Scrolling support for long content
/// - Project/label selection interfaces
/// - Search functionality with live results
/// - Integration with sync service for immediate updates
/// - Configurable display options
///
/// The component delegates specific dialog rendering and logic to specialized
/// dialog modules while providing common infrastructure like input handling
/// and state management.
pub struct DialogComponent {
    pub dialog_type: Option<DialogType>,
    pub input_buffer: String,
    pub cursor_position: usize,
    pub projects: Vec<ProjectDisplay>,
    pub labels: Vec<LabelDisplay>,
    pub tasks: Vec<TaskDisplay>,
    pub selected_project_index: usize,
    pub selected_parent_project_index: Option<usize>, // For project creation parent selection
    pub selected_task_project_index: Option<usize>,   // For task creation project selection (None = no project/inbox)
    pub icons: IconService,
    // Scrolling support for long content dialogs
    pub scroll_offset: usize,
    pub scrollbar_state: ScrollbarState,
    // Task search state
    pub search_results: Vec<TaskDisplay>,
    pub sync_service: Option<SyncService>,
    pub display_config: DisplayConfig,
}

impl Default for DialogComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl DialogComponent {
    pub fn new() -> Self {
        Self {
            dialog_type: None,
            input_buffer: String::new(),
            cursor_position: 0,
            projects: Vec::new(),
            labels: Vec::new(),
            tasks: Vec::new(),
            selected_project_index: 0,
            selected_parent_project_index: None,
            selected_task_project_index: None, // Default to "None" for tasks (no project)
            icons: IconService::default(),
            scroll_offset: 0,
            scrollbar_state: ScrollbarState::new(0),
            search_results: Vec::new(),
            sync_service: None,
            display_config: DisplayConfig::default(),
        }
    }

    pub fn update_display_config(&mut self, display_config: DisplayConfig) {
        self.display_config = display_config;
    }

    pub fn update_data(&mut self, projects: Vec<ProjectDisplay>, labels: Vec<LabelDisplay>) {
        self.projects = projects;
        self.labels = labels;
    }

    pub fn update_data_with_tasks(
        &mut self,
        projects: Vec<ProjectDisplay>,
        labels: Vec<LabelDisplay>,
        tasks: Vec<TaskDisplay>,
    ) {
        self.projects = projects;
        self.labels = labels;
        self.tasks = tasks;
    }

    pub fn set_sync_service(&mut self, sync_service: SyncService) {
        self.sync_service = Some(sync_service);
    }

    /// Get root projects (projects without a parent) for parent selection
    pub fn get_root_projects(&self) -> Vec<&ProjectDisplay> {
        self.projects.iter().filter(|project| project.parent_id.is_none()).collect()
    }

    /// Get all non-inbox projects for task creation (excludes inbox project)
    pub fn get_task_projects(&self) -> Vec<&ProjectDisplay> {
        self.projects.iter().filter(|project| !project.is_inbox_project).collect()
    }

    /// Trigger a database search based on current input
    fn trigger_search(&mut self) -> Action {
        // Trigger background database search
        Action::SearchTasks(self.input_buffer.clone())
    }

    /// Update search results from database query results
    pub fn update_search_results(&mut self, query: &str, results: Vec<TaskDisplay>) {
        // Only update if this is for the current query (avoid race conditions)
        if query == self.input_buffer {
            self.search_results = results;
        }
    }

    pub fn is_visible(&self) -> bool {
        self.dialog_type.is_some()
    }

    fn handle_submit(&mut self) -> Action {
        match &self.dialog_type {
            Some(DialogType::TaskCreation { default_project_id }) => {
                if !self.input_buffer.is_empty() {
                    // Use the user's selection first, fallback to default project ID if no selection made
                    let project_id = if let Some(task_index) = self.selected_task_project_index {
                        let task_projects = self.get_task_projects();
                        if task_index < task_projects.len() {
                            Some(task_projects[task_index].id.clone())
                        } else {
                            None // Task goes to inbox (no project)
                        }
                    } else {
                        // If no user selection, use the default project ID or None
                        default_project_id.clone()
                    };

                    let action = Action::CreateTask {
                        content: self.input_buffer.clone(),
                        project_id,
                    };
                    self.clear_dialog();
                    action
                } else {
                    Action::None
                }
            }
            Some(DialogType::TaskEdit { task_id, .. }) => {
                if !self.input_buffer.is_empty() {
                    let action = Action::EditTask {
                        id: task_id.clone(),
                        content: self.input_buffer.clone(),
                    };
                    self.clear_dialog();
                    action
                } else {
                    Action::None
                }
            }
            Some(DialogType::ProjectCreation) => {
                if !self.input_buffer.is_empty() {
                    let parent_id = if let Some(parent_index) = self.selected_parent_project_index {
                        let root_projects = self.get_root_projects();
                        if parent_index < root_projects.len() {
                            Some(root_projects[parent_index].id.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    let action = Action::CreateProject {
                        name: self.input_buffer.clone(),
                        parent_id,
                    };
                    self.clear_dialog();
                    action
                } else {
                    Action::None
                }
            }
            Some(DialogType::ProjectEdit { project_id, .. }) => {
                if !self.input_buffer.is_empty() {
                    let action = Action::EditProject {
                        id: project_id.clone(),
                        name: self.input_buffer.clone(),
                    };
                    self.clear_dialog();
                    action
                } else {
                    Action::None
                }
            }
            Some(DialogType::LabelCreation) => {
                if !self.input_buffer.is_empty() {
                    let action = Action::CreateLabel {
                        name: self.input_buffer.clone(),
                    };
                    self.clear_dialog();
                    action
                } else {
                    Action::None
                }
            }
            Some(DialogType::LabelEdit { label_id, .. }) => {
                if !self.input_buffer.is_empty() {
                    let action = Action::EditLabel {
                        id: label_id.clone(),
                        name: self.input_buffer.clone(),
                    };
                    self.clear_dialog();
                    action
                } else {
                    Action::None
                }
            }
            Some(DialogType::DeleteConfirmation { item_type, item_id }) => match item_type.as_str() {
                "task" => {
                    let action = Action::DeleteTask(item_id.clone());
                    self.clear_dialog();
                    action
                }
                "project" => {
                    let action = Action::DeleteProject(item_id.clone());
                    self.clear_dialog();
                    action
                }
                "label" => {
                    let action = Action::DeleteLabel(item_id.clone());
                    self.clear_dialog();
                    action
                }
                _ => Action::None,
            },
            _ => Action::None,
        }
    }

    fn clear_dialog(&mut self) {
        self.dialog_type = None;
        self.input_buffer.clear();
        self.cursor_position = 0;
        self.selected_project_index = 0;
        self.selected_parent_project_index = None;
        self.selected_task_project_index = None; // Reset to "None" for task creation
        self.scroll_offset = 0;
        self.scrollbar_state = ScrollbarState::new(0);
        self.search_results.clear();
    }

    fn scroll_up(&mut self) {
        scroll_behavior::scroll_up(&mut self.scroll_offset, &mut self.scrollbar_state);
    }

    fn scroll_down(&mut self) {
        scroll_behavior::scroll_down(&mut self.scroll_offset, &mut self.scrollbar_state);
    }

    fn page_up(&mut self) {
        scroll_behavior::page_up(&mut self.scroll_offset, &mut self.scrollbar_state);
    }

    fn page_down(&mut self) {
        scroll_behavior::page_down(&mut self.scroll_offset, &mut self.scrollbar_state);
    }

    fn scroll_to_top(&mut self) {
        scroll_behavior::scroll_to_top(&mut self.scroll_offset, &mut self.scrollbar_state);
    }

    fn scroll_to_bottom(&mut self) {
        scroll_behavior::scroll_to_bottom(&mut self.scroll_offset, &mut self.scrollbar_state);
    }

    fn render_task_creation_dialog(&self, f: &mut Frame, area: Rect) {
        let task_projects = self.get_task_projects();
        task_dialogs::render_task_creation_dialog(
            f,
            area,
            &self.icons,
            &self.input_buffer,
            &task_projects,
            self.selected_task_project_index,
        );
    }

    fn render_project_creation_dialog(&self, f: &mut Frame, area: Rect) {
        let root_projects = self.get_root_projects();
        project_dialogs::render_project_creation_dialog(
            f,
            area,
            &self.icons,
            &self.input_buffer,
            &root_projects,
            self.selected_parent_project_index,
        );
    }

    fn render_project_edit_dialog(&self, f: &mut Frame, area: Rect) {
        project_dialogs::render_project_edit_dialog(f, area, &self.icons, &self.input_buffer);
    }

    fn render_label_creation_dialog(&self, f: &mut Frame, area: Rect) {
        label_dialogs::render_label_creation_dialog(f, area, &self.icons, &self.input_buffer);
    }

    fn render_label_edit_dialog(&self, f: &mut Frame, area: Rect) {
        label_dialogs::render_label_edit_dialog(f, area, &self.icons, &self.input_buffer);
    }

    fn render_task_edit_dialog(&self, f: &mut Frame, area: Rect) {
        let task_projects = self.get_task_projects();

        // Find the current project index for the task being edited
        let current_project_index = if let Some(DialogType::TaskEdit { project_id, .. }) = &self.dialog_type {
            task_projects.iter().position(|p| p.id == *project_id)
        } else {
            None
        };

        task_dialogs::render_task_edit_dialog(
            f,
            area,
            &self.icons,
            &self.input_buffer,
            &task_projects,
            current_project_index,
        );
    }

    fn render_delete_confirmation_dialog(&self, f: &mut Frame, area: Rect, item_type: &str) {
        system_dialogs::render_delete_confirmation_dialog(f, area, &self.icons, item_type);
    }

    fn render_info_dialog(&mut self, f: &mut Frame, area: Rect, message: &str) {
        system_dialogs::render_info_dialog(
            f,
            area,
            &self.icons,
            message,
            self.scroll_offset,
            &mut self.scrollbar_state,
        );
    }

    fn render_error_dialog(&mut self, f: &mut Frame, area: Rect, message: &str) {
        system_dialogs::render_error_dialog(
            f,
            area,
            &self.icons,
            message,
            self.scroll_offset,
            &mut self.scrollbar_state,
        );
    }

    fn render_help_dialog(&mut self, f: &mut Frame, area: Rect) {
        system_dialogs::render_help_dialog(f, area, self.scroll_offset, &mut self.scrollbar_state);
    }

    fn render_task_search_dialog(&self, f: &mut Frame, area: Rect) {
        use ratatui::{
            layout::{Constraint, Layout, Margin},
            style::{Color, Style},
            widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
        };

        // Create a centered popup area
        let popup_area = {
            let popup_layout =
                Layout::vertical([Constraint::Percentage(10), Constraint::Min(20), Constraint::Percentage(10)])
                    .split(area);

            Layout::horizontal([Constraint::Percentage(10), Constraint::Min(60), Constraint::Percentage(10)])
                .split(popup_layout[1])[1]
        };

        // Clear the area
        f.render_widget(Clear, popup_area);

        // Split into input area and results area
        let content_area = popup_area.inner(Margin {
            horizontal: 1,
            vertical: 1,
        });

        let layout = Layout::vertical([
            Constraint::Length(3), // Input area
            Constraint::Min(0),    // Results area
        ])
        .split(content_area);

        // Render the main block
        let main_block = Block::default()
            .title(" Search Tasks ")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Gray));
        f.render_widget(main_block, popup_area);

        // Render input field
        let input_paragraph = Paragraph::new(self.input_buffer.as_str()).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Query")
                .style(Style::default().fg(Color::Gray)),
        );
        f.render_widget(input_paragraph, layout[0]);

        // Set cursor position in input field
        if !self.input_buffer.is_empty() || self.cursor_position == 0 {
            f.set_cursor_position((layout[0].x + 1 + self.cursor_position as u16, layout[0].y + 1));
        }

        // Render search results
        let results_text = if self.search_results.is_empty() {
            if self.input_buffer.is_empty() {
                "Start typing to search tasks…".to_string()
            } else {
                "No tasks found.".to_string()
            }
        } else {
            format!("{} tasks found", self.search_results.len())
        };

        let results_list: Vec<ListItem> = self
            .search_results
            .iter()
            .map(|task| {
                // Create TaskItem with the same formatting as main task list
                let task_item = TaskItem::new(
                    task.clone(),
                    0, // depth: 0 for search results (no indentation)
                    0, // child_count: 0 for search results
                    self.icons.clone(),
                    self.projects.clone(),
                );

                // Use the same render method as main task list
                TaskListItem::render(&task_item, false, &self.display_config)
            })
            .collect();

        let results_block = Block::default()
            .borders(Borders::ALL)
            .title(results_text)
            .style(Style::default().fg(Color::Gray));

        let results_list_widget = List::new(results_list).block(results_block);
        f.render_widget(results_list_widget, layout[1]);
    }

    fn render_logs_dialog(&mut self, f: &mut Frame, area: Rect) {
        system_dialogs::render_logs_dialog(f, area, self.scroll_offset, &mut self.scrollbar_state);
    }
}

impl Component for DialogComponent {
    fn handle_key_events(&mut self, key: KeyEvent) -> Action {
        if self.dialog_type.is_none() {
            return Action::None;
        }

        match &self.dialog_type {
            Some(DialogType::Info(_)) | Some(DialogType::Error(_)) => {
                // Info/error dialogs with scrolling support
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.scroll_up();
                        Action::None
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.scroll_down();
                        Action::None
                    }
                    KeyCode::PageUp => {
                        self.page_up();
                        Action::None
                    }
                    KeyCode::PageDown => {
                        self.page_down();
                        Action::None
                    }
                    KeyCode::Home => {
                        self.scroll_to_top();
                        Action::None
                    }
                    KeyCode::End => {
                        self.scroll_to_bottom();
                        Action::None
                    }
                    _ => Action::HideDialog, // Any other key dismisses the dialog
                }
            }
            Some(DialogType::Help) => {
                // Help dialog with scrolling support
                match key.code {
                    KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('h') => Action::HideDialog,
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.scroll_up();
                        Action::None
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.scroll_down();
                        Action::None
                    }
                    KeyCode::PageUp => {
                        self.page_up();
                        Action::None
                    }
                    KeyCode::PageDown => {
                        self.page_down();
                        Action::None
                    }
                    KeyCode::Home => {
                        self.scroll_to_top();
                        Action::None
                    }
                    KeyCode::End => {
                        self.scroll_to_bottom();
                        Action::None
                    }
                    _ => Action::None,
                }
            }
            Some(DialogType::Logs) => {
                // Logs dialog with scrolling support (same as help dialog)
                match key.code {
                    KeyCode::Esc | KeyCode::Char('G') | KeyCode::Char('q') => Action::HideDialog,
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.scroll_up();
                        Action::None
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.scroll_down();
                        Action::None
                    }
                    KeyCode::PageUp => {
                        self.page_up();
                        Action::None
                    }
                    KeyCode::PageDown => {
                        self.page_down();
                        Action::None
                    }
                    KeyCode::Home => {
                        self.scroll_to_top();
                        Action::None
                    }
                    KeyCode::End => {
                        self.scroll_to_bottom();
                        Action::None
                    }
                    _ => Action::None,
                }
            }
            Some(DialogType::DeleteConfirmation { .. }) => match key.code {
                KeyCode::Esc => Action::HideDialog,
                KeyCode::Enter => self.handle_submit(),
                _ => Action::None,
            },
            Some(DialogType::TaskSearch) => match key.code {
                KeyCode::Esc => Action::HideDialog,
                KeyCode::Enter => Action::HideDialog,
                KeyCode::Char(c) => {
                    self.input_buffer.insert(self.cursor_position, c);
                    self.cursor_position += 1;
                    self.trigger_search()
                }
                KeyCode::Backspace => {
                    if self.cursor_position > 0 {
                        self.input_buffer.remove(self.cursor_position - 1);
                        self.cursor_position -= 1;
                        return self.trigger_search();
                    }
                    Action::None
                }
                KeyCode::Delete => {
                    if self.cursor_position < self.input_buffer.len() {
                        self.input_buffer.remove(self.cursor_position);
                        return self.trigger_search();
                    }
                    Action::None
                }
                KeyCode::Left => {
                    if self.cursor_position > 0 {
                        self.cursor_position -= 1;
                    }
                    Action::None
                }
                KeyCode::Right => {
                    if self.cursor_position < self.input_buffer.len() {
                        self.cursor_position += 1;
                    }
                    Action::None
                }
                _ => Action::None,
            },
            _ => {
                // Input dialogs
                match key.code {
                    KeyCode::Esc => Action::HideDialog,
                    KeyCode::Enter => self.handle_submit(),
                    KeyCode::Char(c) => {
                        self.input_buffer.insert(self.cursor_position, c);
                        self.cursor_position += 1;
                        Action::None
                    }
                    KeyCode::Backspace => {
                        if self.cursor_position > 0 {
                            self.input_buffer.remove(self.cursor_position - 1);
                            self.cursor_position -= 1;
                        }
                        Action::None
                    }
                    KeyCode::Delete => {
                        if self.cursor_position < self.input_buffer.len() {
                            self.input_buffer.remove(self.cursor_position);
                        }
                        Action::None
                    }
                    KeyCode::Left => {
                        if self.cursor_position > 0 {
                            self.cursor_position -= 1;
                        }
                        Action::None
                    }
                    KeyCode::Right => {
                        if self.cursor_position < self.input_buffer.len() {
                            self.cursor_position += 1;
                        }
                        Action::None
                    }
                    KeyCode::Tab => {
                        if matches!(self.dialog_type, Some(DialogType::TaskCreation { .. })) {
                            let task_projects = self.get_task_projects();
                            if !task_projects.is_empty() {
                                self.selected_task_project_index = match self.selected_task_project_index {
                                    None => Some(0), // First tab: select first project
                                    Some(index) => {
                                        let next_index = (index + 1) % (task_projects.len() + 1);
                                        if next_index == task_projects.len() {
                                            None // Cycle back to "None" option
                                        } else {
                                            Some(next_index)
                                        }
                                    }
                                };
                            }
                        } else if matches!(self.dialog_type, Some(DialogType::ProjectCreation)) {
                            let root_projects = self.get_root_projects();
                            if !root_projects.is_empty() {
                                self.selected_parent_project_index = match self.selected_parent_project_index {
                                    None => Some(0), // First tab: select first parent
                                    Some(index) => {
                                        let next_index = (index + 1) % (root_projects.len() + 1);
                                        if next_index == root_projects.len() {
                                            None // Cycle back to "None" option
                                        } else {
                                            Some(next_index)
                                        }
                                    }
                                };
                            }
                        }
                        Action::None
                    }
                    _ => Action::None,
                }
            }
        }
    }

    fn update(&mut self, action: Action) -> Action {
        match action {
            Action::ShowDialog(dialog_type) => {
                // Check if this is a task creation dialog before moving the value
                let is_task_creation = matches!(dialog_type, DialogType::TaskCreation { .. });

                // Pre-populate input for edit dialogs
                match &dialog_type {
                    DialogType::TaskEdit { content, .. } => {
                        self.input_buffer = content.clone();
                        self.cursor_position = content.len();
                    }
                    DialogType::ProjectEdit { name, .. } => {
                        self.input_buffer = name.clone();
                        self.cursor_position = name.len();
                    }
                    DialogType::LabelEdit { name, .. } => {
                        self.input_buffer = name.clone();
                        self.cursor_position = name.len();
                    }
                    DialogType::TaskCreation { default_project_id } => {
                        self.input_buffer.clear();
                        self.cursor_position = 0;
                        // Set the selected task project index if a default project is provided
                        if let Some(project_id) = default_project_id {
                            let task_projects = self.get_task_projects();
                            if let Some(index) = task_projects.iter().position(|p| &p.id == project_id) {
                                self.selected_task_project_index = Some(index);
                            }
                        }
                    }
                    DialogType::TaskSearch => {
                        self.input_buffer.clear();
                        self.cursor_position = 0;
                        self.search_results.clear();
                    }
                    _ => {
                        self.input_buffer.clear();
                        self.cursor_position = 0;
                    }
                }
                self.dialog_type = Some(dialog_type.clone());
                // Only reset project index for non-task-creation dialogs
                if !is_task_creation {
                    self.selected_project_index = 0;
                }

                // Trigger initial search for TaskSearch dialog
                if matches!(dialog_type, DialogType::TaskSearch) {
                    return self.trigger_search();
                }

                Action::None
            }
            Action::HideDialog => {
                self.clear_dialog();
                Action::None
            }
            _ => action,
        }
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        if let Some(dialog_type) = self.dialog_type.clone() {
            match dialog_type {
                DialogType::TaskCreation { .. } => self.render_task_creation_dialog(f, rect),
                DialogType::TaskEdit { .. } => self.render_task_edit_dialog(f, rect),
                DialogType::ProjectCreation => {
                    self.render_project_creation_dialog(f, rect);
                }
                DialogType::ProjectEdit { .. } => {
                    self.render_project_edit_dialog(f, rect);
                }
                DialogType::LabelCreation => {
                    self.render_label_creation_dialog(f, rect);
                }
                DialogType::LabelEdit { .. } => {
                    self.render_label_edit_dialog(f, rect);
                }
                DialogType::DeleteConfirmation { item_type, .. } => {
                    self.render_delete_confirmation_dialog(f, rect, &item_type);
                }
                DialogType::Info(message) => {
                    self.render_info_dialog(f, rect, &message);
                }
                DialogType::Error(message) => {
                    self.render_error_dialog(f, rect, &message);
                }
                DialogType::Help => {
                    self.render_help_dialog(f, rect);
                }
                DialogType::Logs => {
                    self.render_logs_dialog(f, rect);
                }
                DialogType::TaskSearch => {
                    self.render_task_search_dialog(f, rect);
                }
            }
        }
    }
}
