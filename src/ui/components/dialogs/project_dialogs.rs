use super::common::{self, shortcuts};
use crate::icons::IconService;
use crate::ui::layout::LayoutManager;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    widgets::Clear,
    Frame,
};

pub fn render_project_creation_dialog(
    f: &mut Frame,
    area: Rect,
    _icons: &IconService,
    input_buffer: &str,
    root_projects: &[&crate::entities::project::Model],
    selected_parent_index: Option<usize>,
) {
    let dialog_area = LayoutManager::centered_rect_lines(65, 12, area);
    f.render_widget(Clear, dialog_area);

    let main_block = common::create_dialog_block("New Project", Color::Magenta);

    // Create layout for content
    let inner_area = main_block.inner(dialog_area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(4), // Project name input field (borders + content)
            Constraint::Length(4), // Parent project field (borders + content)
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Instructions
        ])
        .split(inner_area);

    let input_paragraph = common::create_input_paragraph(input_buffer, "Project Name");

    // Parent project selection field
    let parent_project_name = match selected_parent_index {
        None => "None (Root Project)".to_string(),
        Some(index) => {
            if index < root_projects.len() {
                root_projects[index].name.clone()
            } else {
                "None (Root Project)".to_string()
            }
        }
    };

    let parent_paragraph = common::create_selection_paragraph(parent_project_name, "Parent Project");

    let instructions = [
        ("Enter", Color::Green, " Create Project"),
        shortcuts::SEPARATOR,
        shortcuts::TAB_SELECT,
        (" Parent", Color::Gray, ""),
        shortcuts::SEPARATOR,
        shortcuts::ESC_CANCEL,
    ];
    let instructions_paragraph = common::create_instructions_paragraph(&instructions);

    // Render all components
    f.render_widget(main_block, dialog_area);
    f.render_widget(input_paragraph, chunks[0]);
    f.render_widget(parent_paragraph, chunks[1]);
    f.render_widget(instructions_paragraph, chunks[3]);
}

pub fn render_project_edit_dialog(f: &mut Frame, area: Rect, _icons: &IconService, input_buffer: &str) {
    let dialog_area = LayoutManager::centered_rect_lines(65, 9, area);
    f.render_widget(Clear, dialog_area);

    let main_block = common::create_dialog_block("Edit Project", Color::Yellow);

    // Create layout for content
    let inner_area = main_block.inner(dialog_area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(4), // Input field (borders + content)
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Instructions
        ])
        .split(inner_area);

    let input_paragraph = common::create_input_paragraph(input_buffer, "Project Name");

    let instructions = [
        ("Enter", Color::Green, " Save Changes"),
        shortcuts::SEPARATOR,
        shortcuts::ESC_CANCEL,
    ];
    let instructions_paragraph = common::create_instructions_paragraph(&instructions);

    // Render all components
    f.render_widget(main_block, dialog_area);
    f.render_widget(input_paragraph, chunks[0]);
    f.render_widget(instructions_paragraph, chunks[2]);
}
