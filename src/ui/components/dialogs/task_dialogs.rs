use super::common::{self, shortcuts};
use crate::entities::project;
use crate::icons::IconService;
use crate::ui::layout::LayoutManager;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    widgets::Clear,
    Frame,
};

#[allow(clippy::too_many_arguments)]
pub fn render_task_dialog(
    f: &mut Frame,
    area: Rect,
    _icons: &IconService,
    input_buffer: &str,
    cursor_position: usize,
    task_projects: &[&project::Model],
    selected_project_index: Option<usize>,
    is_editing: bool,
) {
    let title = if is_editing { "Edit Task" } else { "New Task" };
    let dialog_area = LayoutManager::centered_rect_lines(65, 12, area);
    f.render_widget(Clear, dialog_area);

    let main_block = common::create_dialog_block(title, Color::Cyan);

    // Create layout for content
    let inner_area = main_block.inner(dialog_area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(4), // Task content input field (borders + content)
            Constraint::Length(4), // Project selection field (borders + content)
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Instructions
        ])
        .split(inner_area);

    let input_width = chunks[0].width.saturating_sub(2);
    let input_paragraph = common::create_input_paragraph(input_buffer, cursor_position, input_width, "Task Content");

    // Project selection field
    let project_name = match selected_project_index {
        None => "None (Inbox)".to_string(),
        Some(index) => {
            if index < task_projects.len() {
                task_projects[index].name.clone()
            } else {
                "None (Inbox)".to_string()
            }
        }
    };

    let project_paragraph = common::create_selection_paragraph(project_name, "Project");

    // Instructions based on mode
    let action = if is_editing {
        ("Enter", Color::Green, " Save Task")
    } else {
        ("Enter", Color::Green, " Create Task")
    };

    let instructions = [
        action,
        shortcuts::SEPARATOR,
        shortcuts::TAB_SELECT,
        (" Project", Color::Gray, ""),
        shortcuts::SEPARATOR,
        shortcuts::ESC_CANCEL,
    ];
    let instructions_paragraph = common::create_instructions_paragraph(&instructions);

    // Render all components
    f.render_widget(main_block, dialog_area);
    f.render_widget(input_paragraph, chunks[0]);
    f.render_widget(project_paragraph, chunks[1]);
    f.render_widget(instructions_paragraph, chunks[3]);

    // Set the cursor inside the horizontally scrolled input viewport.
    let (_, visible_cursor_column) = common::input_viewport(input_buffer, cursor_position, input_width);
    f.set_cursor_position((chunks[0].x + 1 + visible_cursor_column, chunks[0].y + 1));
}

// Legacy wrapper functions for backward compatibility
pub fn render_task_creation_dialog(
    f: &mut Frame,
    area: Rect,
    icons: &IconService,
    input_buffer: &str,
    cursor_position: usize,
    task_projects: &[&project::Model],
    selected_task_project_index: Option<usize>,
) {
    render_task_dialog(
        f,
        area,
        icons,
        input_buffer,
        cursor_position,
        task_projects,
        selected_task_project_index,
        false, // is_editing = false for creation
    );
}

pub fn render_task_edit_dialog(
    f: &mut Frame,
    area: Rect,
    icons: &IconService,
    input_buffer: &str,
    cursor_position: usize,
    task_projects: &[&project::Model],
    selected_task_project_index: Option<usize>,
) {
    render_task_dialog(
        f,
        area,
        icons,
        input_buffer,
        cursor_position,
        task_projects,
        selected_task_project_index,
        true, // is_editing = true for editing
    );
}

pub fn render_task_time_dialog(f: &mut Frame, area: Rect, input: &str, cursor_position: usize) {
    let dialog_area = LayoutManager::centered_rect_lines(45, 8, area);
    f.render_widget(Clear, dialog_area);
    let block = common::create_dialog_block("Set Due Time", Color::Cyan);
    let inner = block.inner(dialog_area);
    let chunks = Layout::vertical([Constraint::Length(4), Constraint::Length(1), Constraint::Length(1)])
        .margin(1)
        .split(inner);
    let input_width = chunks[0].width.saturating_sub(2);
    let field = common::create_input_paragraph(input, cursor_position, input_width, "Time (e.g. 2pm)");
    let instructions = common::create_instructions_paragraph(&[
        ("Enter", Color::Green, " Save"),
        shortcuts::SEPARATOR,
        shortcuts::ESC_CANCEL,
    ]);
    f.render_widget(block, dialog_area);
    f.render_widget(field, chunks[0]);
    f.render_widget(instructions, chunks[2]);
    let (_, cursor) = common::input_viewport(input, cursor_position, input_width);
    f.set_cursor_position((chunks[0].x + 1 + cursor, chunks[0].y + 1));
}
