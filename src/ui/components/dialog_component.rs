use crate::debug_logger::DebugLogger;
use crate::icons::IconService;
use crate::todoist::{LabelDisplay, ProjectDisplay};
use crate::ui::core::{
    actions::{Action, DialogType},
    Component,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, widgets::ScrollbarState, Frame};

use crate::ui::components::dialogs::{label_dialogs, project_dialogs, scroll_behavior, system_dialogs, task_dialogs};

pub struct DialogComponent {
    pub dialog_type: Option<DialogType>,
    pub input_buffer: String,
    pub cursor_position: usize,
    pub projects: Vec<ProjectDisplay>,
    pub labels: Vec<LabelDisplay>,
    pub selected_project_index: usize,
    pub selected_parent_project_index: Option<usize>, // For project creation parent selection
    pub icons: IconService,
    // Scrolling support for long content dialogs
    pub scroll_offset: usize,
    pub scrollbar_state: ScrollbarState,
    // Debug logger for logs dialog
    pub debug_logger: Option<DebugLogger>,
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
            selected_project_index: 0,
            selected_parent_project_index: None,
            icons: IconService::default(),
            scroll_offset: 0,
            scrollbar_state: ScrollbarState::new(0),
            debug_logger: None,
        }
    }

    pub fn update_data(&mut self, projects: Vec<ProjectDisplay>, labels: Vec<LabelDisplay>) {
        self.projects = projects;
        self.labels = labels;
    }

    /// Get root projects (projects without a parent) for parent selection
    pub fn get_root_projects(&self) -> Vec<&ProjectDisplay> {
        self.projects
            .iter()
            .filter(|project| project.parent_id.is_none())
            .collect()
    }

    pub fn set_debug_logger(&mut self, logger: DebugLogger) {
        self.debug_logger = Some(logger);
    }

    pub fn is_visible(&self) -> bool {
        self.dialog_type.is_some()
    }

    fn handle_submit(&mut self) -> Action {
        match &self.dialog_type {
            Some(DialogType::TaskCreation { default_project_id }) => {
                if !self.input_buffer.is_empty() {
                    // Use the default project ID if provided, otherwise use selected project
                    let project_id = default_project_id.clone().or_else(|| {
                        if !self.projects.is_empty() {
                            Some(self.projects[self.selected_project_index].id.clone())
                        } else {
                            None
                        }
                    });

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
        self.scroll_offset = 0;
        self.scrollbar_state = ScrollbarState::new(0);
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
        task_dialogs::render_task_creation_dialog(
            f,
            area,
            &self.icons,
            &self.input_buffer,
            &self.projects,
            self.selected_project_index,
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
        task_dialogs::render_task_edit_dialog(f, area, &self.icons, &self.input_buffer);
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

    fn render_logs_dialog(&mut self, f: &mut Frame, area: Rect) {
        system_dialogs::render_logs_dialog(
            f,
            area,
            &self.debug_logger,
            self.scroll_offset,
            &mut self.scrollbar_state,
        );
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
                        if matches!(self.dialog_type, Some(DialogType::TaskCreation { .. }))
                            && !self.projects.is_empty()
                        {
                            self.selected_project_index = (self.selected_project_index + 1) % self.projects.len();
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
                        // Set the selected project index if a default project is provided
                        if let Some(project_id) = default_project_id {
                            if let Some(index) = self.projects.iter().position(|p| &p.id == project_id) {
                                self.selected_project_index = index;
                            }
                        }
                    }
                    _ => {
                        self.input_buffer.clear();
                        self.cursor_position = 0;
                    }
                }
                self.dialog_type = Some(dialog_type);
                // Only reset project index for non-task-creation dialogs
                if !is_task_creation {
                    self.selected_project_index = 0;
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
            }
        }
    }
}
