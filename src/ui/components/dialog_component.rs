use crate::debug_logger::DebugLogger;
use crate::icons::IconService;
use crate::todoist::{LabelDisplay, ProjectDisplay};
use crate::ui::core::{
    actions::{Action, DialogType},
    Component,
};
use crate::ui::layout::LayoutManager;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

pub struct DialogComponent {
    pub dialog_type: Option<DialogType>,
    pub input_buffer: String,
    pub cursor_position: usize,
    pub projects: Vec<ProjectDisplay>,
    pub labels: Vec<LabelDisplay>,
    pub selected_project_index: usize,
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
                    let action = Action::CreateProject {
                        name: self.input_buffer.clone(),
                        parent_id: None,
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
        self.scroll_offset = 0;
        self.scrollbar_state = ScrollbarState::new(0);
    }

    // Scrolling methods for long content dialogs
    fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
        self.scrollbar_state = self.scrollbar_state.position(self.scroll_offset);
    }

    fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
        self.scrollbar_state = self.scrollbar_state.position(self.scroll_offset);
    }

    fn page_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(10);
        self.scrollbar_state = self.scrollbar_state.position(self.scroll_offset);
    }

    fn page_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(10);
        self.scrollbar_state = self.scrollbar_state.position(self.scroll_offset);
    }

    fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
        self.scrollbar_state = self.scrollbar_state.position(self.scroll_offset);
    }

    fn scroll_to_bottom(&mut self) {
        self.scroll_offset = usize::MAX; // Will be clamped in render
        self.scrollbar_state = self.scrollbar_state.position(self.scroll_offset);
    }

    fn render_task_creation_dialog(&self, f: &mut Frame, area: Rect) {
        let dialog_area = LayoutManager::centered_rect(60, 20, area);
        f.render_widget(Clear, dialog_area);

        let title = format!("{} Create New Task", self.icons.info());
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().fg(Color::White));

        // Task content input
        let content_text = format!("Content: {}", self.input_buffer);
        let content_paragraph = Paragraph::new(content_text)
            .block(Block::default().borders(Borders::ALL).title("Task Content"))
            .style(Style::default().fg(Color::White));

        // Project selection
        let project_name = if !self.projects.is_empty() {
            &self.projects[self.selected_project_index].name
        } else {
            "No projects available"
        };
        let project_text = format!("Project: {}", project_name);
        let project_paragraph = Paragraph::new(project_text)
            .block(Block::default().borders(Borders::ALL).title("Project"))
            .style(Style::default().fg(Color::White));

        // Instructions
        let instructions = "Press Enter to create, Esc to cancel, Tab to change project";
        let instructions_paragraph = Paragraph::new(instructions)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        // Split dialog area
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(3), // Content
                ratatui::layout::Constraint::Length(3), // Project
                ratatui::layout::Constraint::Length(1), // Instructions
            ])
            .split(dialog_area);

        f.render_widget(block, dialog_area);
        f.render_widget(content_paragraph, chunks[0]);
        f.render_widget(project_paragraph, chunks[1]);
        f.render_widget(instructions_paragraph, chunks[2]);
    }

    fn render_project_creation_dialog(&self, f: &mut Frame, area: Rect) {
        let dialog_area = LayoutManager::centered_rect(60, 15, area);
        f.render_widget(Clear, dialog_area);

        let title = format!("{} Create New Project", self.icons.info());
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().fg(Color::White));

        // Project name input
        let content_text = format!("Project Name: {}", self.input_buffer);
        let content_paragraph = Paragraph::new(content_text)
            .block(Block::default().borders(Borders::ALL).title("Project Name"))
            .style(Style::default().fg(Color::White));

        // Instructions
        let instructions = "Press Enter to create, Esc to cancel";
        let instructions_paragraph = Paragraph::new(instructions)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        // Split dialog area
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(3), // Project name
                ratatui::layout::Constraint::Length(1), // Instructions
            ])
            .split(dialog_area);

        f.render_widget(block, dialog_area);
        f.render_widget(content_paragraph, chunks[0]);
        f.render_widget(instructions_paragraph, chunks[1]);
    }

    fn render_project_edit_dialog(&self, f: &mut Frame, area: Rect) {
        let dialog_area = LayoutManager::centered_rect(60, 15, area);
        f.render_widget(Clear, dialog_area);

        let title = format!("{} Edit Project", self.icons.info());
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().fg(Color::White));

        // Project name input
        let content_text = format!("Project Name: {}", self.input_buffer);
        let content_paragraph = Paragraph::new(content_text)
            .block(Block::default().borders(Borders::ALL).title("Project Name"))
            .style(Style::default().fg(Color::White));

        // Instructions
        let instructions = "Press Enter to save, Esc to cancel";
        let instructions_paragraph = Paragraph::new(instructions)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        // Split dialog area
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(3), // Project name
                ratatui::layout::Constraint::Length(1), // Instructions
            ])
            .split(dialog_area);

        f.render_widget(block, dialog_area);
        f.render_widget(content_paragraph, chunks[0]);
        f.render_widget(instructions_paragraph, chunks[1]);
    }

    fn render_label_creation_dialog(&self, f: &mut Frame, area: Rect) {
        let dialog_area = LayoutManager::centered_rect(60, 15, area);
        f.render_widget(Clear, dialog_area);

        let title = format!("{} Create New Label", self.icons.info());
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().fg(Color::White));

        // Label name input
        let content_text = format!("Label Name: {}", self.input_buffer);
        let content_paragraph = Paragraph::new(content_text)
            .block(Block::default().borders(Borders::ALL).title("Label Name"))
            .style(Style::default().fg(Color::White));

        // Instructions
        let instructions = "Press Enter to create, Esc to cancel";
        let instructions_paragraph = Paragraph::new(instructions)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        // Split dialog area
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(3), // Label name
                ratatui::layout::Constraint::Length(1), // Instructions
            ])
            .split(dialog_area);

        f.render_widget(block, dialog_area);
        f.render_widget(content_paragraph, chunks[0]);
        f.render_widget(instructions_paragraph, chunks[1]);
    }

    fn render_label_edit_dialog(&self, f: &mut Frame, area: Rect) {
        let dialog_area = LayoutManager::centered_rect(60, 15, area);
        f.render_widget(Clear, dialog_area);

        let title = format!("{} Edit Label", self.icons.info());
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().fg(Color::White));

        // Label name input
        let content_text = format!("Label Name: {}", self.input_buffer);
        let content_paragraph = Paragraph::new(content_text)
            .block(Block::default().borders(Borders::ALL).title("Label Name"))
            .style(Style::default().fg(Color::White));

        // Instructions
        let instructions = "Press Enter to save, Esc to cancel";
        let instructions_paragraph = Paragraph::new(instructions)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        // Split dialog area
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(3), // Label name
                ratatui::layout::Constraint::Length(1), // Instructions
            ])
            .split(dialog_area);

        f.render_widget(block, dialog_area);
        f.render_widget(content_paragraph, chunks[0]);
        f.render_widget(instructions_paragraph, chunks[1]);
    }

    fn render_task_edit_dialog(&self, f: &mut Frame, area: Rect) {
        let dialog_area = LayoutManager::centered_rect(60, 15, area);
        f.render_widget(Clear, dialog_area);

        let title = format!("{} Edit Task", self.icons.info());
        let content_text = format!("Content: {}", self.input_buffer);
        let instructions = "Press Enter to save, Esc to cancel";

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().fg(Color::White));

        let content_paragraph = Paragraph::new(content_text)
            .block(Block::default().borders(Borders::ALL).title("Task Content"))
            .style(Style::default().fg(Color::White));

        let instructions_paragraph = Paragraph::new(instructions)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(3),
                ratatui::layout::Constraint::Length(1),
            ])
            .split(dialog_area);

        f.render_widget(block, dialog_area);
        f.render_widget(content_paragraph, chunks[0]);
        f.render_widget(instructions_paragraph, chunks[1]);
    }

    fn render_delete_confirmation_dialog(&self, f: &mut Frame, area: Rect, item_type: &str) {
        let dialog_area = LayoutManager::centered_rect(50, 10, area);
        f.render_widget(Clear, dialog_area);

        let title = format!("{} Confirm Delete", self.icons.warning());
        let message = format!("Are you sure you want to delete this {}?", item_type);
        let instructions = "Press Enter to confirm, Esc to cancel";

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().fg(Color::Red));

        let message_paragraph = Paragraph::new(message)
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center);

        let instructions_paragraph = Paragraph::new(instructions)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(2),
                ratatui::layout::Constraint::Length(1),
            ])
            .split(dialog_area);

        f.render_widget(block, dialog_area);
        f.render_widget(message_paragraph, chunks[0]);
        f.render_widget(instructions_paragraph, chunks[1]);
    }

    fn render_info_dialog(&mut self, f: &mut Frame, area: Rect, message: &str) {
        let dialog_area = LayoutManager::centered_rect(60, 15, area);
        f.render_widget(Clear, dialog_area);

        let title = format!("{} Info", self.icons.info());
        let instructions = "Press any key to continue • j/k to scroll if needed";

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().fg(Color::Blue));

        // Calculate content area for scrolling
        let content_area = Rect::new(
            dialog_area.x + 1,
            dialog_area.y + 1,
            dialog_area.width.saturating_sub(2),
            dialog_area.height.saturating_sub(4), // Leave space for instructions
        );

        let instructions_area = Rect::new(
            dialog_area.x + 1,
            dialog_area.y + dialog_area.height.saturating_sub(2),
            dialog_area.width.saturating_sub(2),
            1,
        );

        // Handle scrolling for long messages
        let lines: Vec<&str> = message.lines().collect();
        let total_lines = lines.len();
        let visible_height = content_area.height as usize;

        let message_text = if total_lines > visible_height {
            // Update scrollbar state
            let max_scroll = total_lines.saturating_sub(visible_height);
            let scroll_offset = self.scroll_offset.min(max_scroll);

            self.scrollbar_state = self
                .scrollbar_state
                .content_length(total_lines)
                .viewport_content_length(visible_height)
                .position(scroll_offset);

            // Extract visible portion
            let visible_lines: Vec<&str> = lines
                .iter()
                .skip(scroll_offset)
                .take(visible_height)
                .copied()
                .collect();
            visible_lines.join("\n")
        } else {
            message.to_string()
        };

        let message_paragraph = Paragraph::new(message_text)
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Left)
            .wrap(ratatui::widgets::Wrap { trim: true });

        let instructions_paragraph = Paragraph::new(instructions)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        f.render_widget(block, dialog_area);
        f.render_widget(message_paragraph, content_area);
        f.render_widget(instructions_paragraph, instructions_area);

        // Render scrollbar if content is scrollable
        if total_lines > visible_height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"))
                .track_symbol(Some("│"))
                .thumb_symbol("▐")
                .style(Style::default().fg(Color::Gray))
                .thumb_style(Style::default().fg(Color::White));

            f.render_stateful_widget(scrollbar, content_area, &mut self.scrollbar_state);
        }
    }

    fn render_error_dialog(&mut self, f: &mut Frame, area: Rect, message: &str) {
        let dialog_area = LayoutManager::centered_rect(70, 15, area);
        f.render_widget(Clear, dialog_area);

        let title = format!("{} Error", self.icons.warning());
        let instructions = "Press any key to continue • j/k to scroll if needed";

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().fg(Color::Red));

        // Calculate content area for scrolling
        let content_area = Rect::new(
            dialog_area.x + 1,
            dialog_area.y + 1,
            dialog_area.width.saturating_sub(2),
            dialog_area.height.saturating_sub(4), // Leave space for instructions
        );

        let instructions_area = Rect::new(
            dialog_area.x + 1,
            dialog_area.y + dialog_area.height.saturating_sub(2),
            dialog_area.width.saturating_sub(2),
            1,
        );

        // Handle scrolling for long error messages
        let lines: Vec<&str> = message.lines().collect();
        let total_lines = lines.len();
        let visible_height = content_area.height as usize;

        let message_text = if total_lines > visible_height {
            // Update scrollbar state
            let max_scroll = total_lines.saturating_sub(visible_height);
            let scroll_offset = self.scroll_offset.min(max_scroll);

            self.scrollbar_state = self
                .scrollbar_state
                .content_length(total_lines)
                .viewport_content_length(visible_height)
                .position(scroll_offset);

            // Extract visible portion
            let visible_lines: Vec<&str> = lines
                .iter()
                .skip(scroll_offset)
                .take(visible_height)
                .copied()
                .collect();
            visible_lines.join("\n")
        } else {
            message.to_string()
        };

        let message_paragraph = Paragraph::new(message_text)
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Left)
            .wrap(ratatui::widgets::Wrap { trim: true });

        let instructions_paragraph = Paragraph::new(instructions)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        f.render_widget(block, dialog_area);
        f.render_widget(message_paragraph, content_area);
        f.render_widget(instructions_paragraph, instructions_area);

        // Render scrollbar if content is scrollable
        if total_lines > visible_height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"))
                .track_symbol(Some("│"))
                .thumb_symbol("▐")
                .style(Style::default().fg(Color::Gray))
                .thumb_style(Style::default().fg(Color::White));

            f.render_stateful_widget(scrollbar, content_area, &mut self.scrollbar_state);
        }
    }

    fn render_help_dialog(&mut self, f: &mut Frame, area: Rect) {
        let help_content = r"
TERMINALIST - Todoist Terminal Client
====================================

NAVIGATION
----------
j/k         Navigate tasks (down/up)
J/K         Navigate projects (down/up)
Enter       Select project/task or confirm action
Esc         Cancel action or close dialogs

PROJECT MANAGEMENT
-----------------
A           Create new project
D           Delete selected project (with confirmation)

TASK MANAGEMENT
--------------
Space       Toggle task completion
a           Create new task
e           Edit selected task
d           Delete task (with confirmation)
t           Set task due date to today
T           Set task due date to tomorrow
w           Set task due date to next week (Monday)
W           Set task due date to next week end (Saturday)

SYNC & DATA
-----------
r           Force sync with Todoist
Ctrl+C      Quit application

GENERAL CONTROLS
----------------
?           Toggle help panel
h           Toggle help panel
q           Quit application
i           Change icon theme

HELP PANEL SCROLLING
--------------------
j/k         Scroll help content down/up
↑↓          Scroll help content up/down
PageUp/Down Page through help content
Home        Jump to top of help
End         Jump to bottom of help

TASK STATUS INDICATORS
----------------------
🔳          Pending task
✅          Completed task
❌          Deleted task

Priority badges: [P1] (red, highest), [P2] (orange), [P3] (blue), [P4] (white, default)

LAYOUT DETAILS
--------------
Left pane:  Projects list with selection
Right pane: Tasks for selected project
Bottom:     Status bar with shortcuts
Help:       Modal overlay with scrollable content

NOTES
-----
Tasks are ordered: pending, then completed, then deleted

Press 'Esc', '?' or 'h' to close this help panel
";

        // Use a large centered rectangle that covers most of the screen
        let help_area = LayoutManager::centered_rect(90, 90, area);
        f.render_widget(Clear, help_area);

        // Calculate help content area with margins
        let margin_x = 2;
        let margin_y = 1;
        let help_content_area = Rect::new(
            help_area.x + margin_x,
            help_area.y + margin_y,
            help_area.width.saturating_sub(margin_x * 2),
            help_area.height.saturating_sub(margin_y * 2),
        );

        // Apply scroll offset to the content
        let lines: Vec<&str> = help_content.lines().collect();
        let total_lines = lines.len();
        let visible_height = help_content_area.height.saturating_sub(2) as usize; // Account for borders

        // Clamp scroll offset to valid range
        let max_scroll = total_lines.saturating_sub(visible_height);
        let scroll_offset = self.scroll_offset.min(max_scroll);

        // Update scrollbar state with current viewport info
        self.scrollbar_state = self
            .scrollbar_state
            .content_length(total_lines)
            .viewport_content_length(visible_height)
            .position(scroll_offset);

        // Extract visible portion of content
        let visible_lines: Vec<&str> = lines
            .iter()
            .skip(scroll_offset)
            .take(visible_height)
            .copied()
            .collect();

        let help_text = visible_lines.join("\n");

        // Create a bordered paragraph for the help content
        let help_paragraph = Paragraph::new(help_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("📖 Help - Press 'Esc', '?' or 'h' to close")
                    .title_alignment(Alignment::Center),
            )
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Left);

        f.render_widget(help_paragraph, help_content_area);

        // Render scrollbar if content is scrollable
        if total_lines > visible_height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"))
                .track_symbol(Some("│"))
                .thumb_symbol("▐")
                .style(Style::default().fg(Color::Gray))
                .thumb_style(Style::default().fg(Color::White));

            f.render_stateful_widget(scrollbar, help_content_area, &mut self.scrollbar_state);
        }
    }

    fn render_logs_dialog(&mut self, f: &mut Frame, area: Rect) {
        // Use a large centered rectangle that covers most of the screen
        let logs_area = LayoutManager::centered_rect(90, 90, area);
        f.render_widget(Clear, logs_area);

        // Calculate logs content area with margins
        let margin_x = 2;
        let margin_y = 1;
        let logs_content_area = Rect::new(
            logs_area.x + margin_x,
            logs_area.y + margin_y,
            logs_area.width.saturating_sub(margin_x * 2),
            logs_area.height.saturating_sub(margin_y * 2),
        );

        // Get logs from debug logger
        let logs = if let Some(ref logger) = self.debug_logger {
            logger.get_logs()
        } else {
            vec!["No debug logger available".to_string()]
        };

        // Convert logs to display format
        let logs_content = if logs.is_empty() {
            "No debug logs available".to_string()
        } else {
            logs.join("\n")
        };

        // Apply scroll offset to the content
        let lines: Vec<&str> = logs_content.lines().collect();
        let total_lines = lines.len();
        let visible_height = logs_content_area.height.saturating_sub(2) as usize; // Account for borders

        // Clamp scroll offset to valid range
        let max_scroll = total_lines.saturating_sub(visible_height);
        let scroll_offset = self.scroll_offset.min(max_scroll);

        // Update scrollbar state with current viewport info
        self.scrollbar_state = self
            .scrollbar_state
            .content_length(total_lines)
            .viewport_content_length(visible_height)
            .position(scroll_offset);

        // Extract visible portion of content
        let visible_lines: Vec<&str> = lines
            .iter()
            .skip(scroll_offset)
            .take(visible_height)
            .copied()
            .collect();

        let logs_text = visible_lines.join("\n");

        // Create a bordered paragraph for the logs content
        let logs_paragraph = Paragraph::new(logs_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("🔍 Debug Logs - Press 'Esc', 'G' or 'q' to close")
                    .title_alignment(Alignment::Center),
            )
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Left);

        f.render_widget(logs_paragraph, logs_content_area);

        // Render scrollbar if content is scrollable
        if total_lines > visible_height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"))
                .track_symbol(Some("│"))
                .thumb_symbol("▐")
                .style(Style::default().fg(Color::Gray))
                .thumb_style(Style::default().fg(Color::White));

            f.render_stateful_widget(scrollbar, logs_content_area, &mut self.scrollbar_state);
        }
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
