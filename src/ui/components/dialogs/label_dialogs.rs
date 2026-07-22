use super::common::{self, shortcuts};
use crate::icons::IconService;
use crate::ui::layout::LayoutManager;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    widgets::Clear,
    Frame,
};

fn render_label_dialog(
    f: &mut Frame,
    area: Rect,
    _icons: &IconService,
    input_buffer: &str,
    cursor_position: usize,
    is_editing: bool,
) {
    let dialog_area = LayoutManager::centered_rect_lines(65, 9, area);
    f.render_widget(Clear, dialog_area);

    let title = if is_editing { "Edit Label" } else { "New Label" };
    let main_block = common::create_dialog_block(title, Color::Cyan);

    // Create layout for content
    let inner_area = main_block.inner(dialog_area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Label name input field (borders + content)
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Instructions
        ])
        .split(inner_area);

    let input_width = chunks[0].width.saturating_sub(2);
    let input_paragraph = common::create_input_paragraph(input_buffer, cursor_position, input_width, "Label Name");

    // Instructions based on mode
    let action = if is_editing {
        ("Enter", Color::Green, " Save Label")
    } else {
        ("Enter", Color::Green, " Create Label")
    };

    let instructions = [action, shortcuts::SEPARATOR, shortcuts::ESC_CANCEL];
    let instructions_paragraph = common::create_instructions_paragraph(&instructions);

    // Render all components
    f.render_widget(main_block, dialog_area);
    f.render_widget(input_paragraph, chunks[0]);
    f.render_widget(instructions_paragraph, chunks[2]);

    // Set the cursor inside the horizontally scrolled input viewport.
    let (_, visible_cursor_column) = common::input_viewport(input_buffer, cursor_position, input_width);
    let final_x = chunks[0].x.saturating_add(1).saturating_add(visible_cursor_column);
    let final_y = chunks[0].y.saturating_add(1);
    f.set_cursor_position((final_x, final_y));
}

pub fn render_label_creation_dialog(
    f: &mut Frame,
    area: Rect,
    icons: &IconService,
    input_buffer: &str,
    cursor_position: usize,
) {
    render_label_dialog(f, area, icons, input_buffer, cursor_position, false);
}

pub fn render_label_edit_dialog(
    f: &mut Frame,
    area: Rect,
    icons: &IconService,
    input_buffer: &str,
    cursor_position: usize,
) {
    render_label_dialog(f, area, icons, input_buffer, cursor_position, true);
}
