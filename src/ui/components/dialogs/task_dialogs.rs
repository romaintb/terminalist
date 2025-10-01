use crate::entities::project;
use crate::icons::IconService;
use crate::ui::layout::LayoutManager;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

pub fn render_task_dialog(
    f: &mut Frame,
    area: Rect,
    _icons: &IconService,
    input_buffer: &str,
    task_projects: &[&project::Model],
    selected_project_index: Option<usize>,
    is_editing: bool,
) {
    let title = if is_editing { "Edit Task" } else { "New Task" };
    let dialog_height = 12;

    let dialog_area = LayoutManager::centered_rect_lines(65, dialog_height, area);
    f.render_widget(Clear, dialog_area);

    // Main dialog block with rounded borders and cyan theme
    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .style(Style::default().fg(Color::Cyan));

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

    // Task content input field with visual cursor
    let cursor_char = "█";
    let input_display = format!("{}{}", input_buffer, cursor_char);

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Task Content ")
        .title_style(Style::default().fg(Color::White))
        .style(Style::default().fg(Color::Gray));

    let input_paragraph = Paragraph::new(input_display)
        .block(input_block)
        .style(Style::default().fg(Color::White));

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

    let project_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Project ")
        .title_style(Style::default().fg(Color::White))
        .style(Style::default().fg(Color::Gray));

    let project_paragraph = Paragraph::new(project_name)
        .block(project_block)
        .style(Style::default().fg(Color::White));

    // Render all components
    f.render_widget(main_block, dialog_area);
    f.render_widget(input_paragraph, chunks[0]);
    f.render_widget(project_paragraph, chunks[1]);

    // Enhanced instructions with color-coded shortcuts
    let action_text = if is_editing { " Save Task" } else { " Create Task" };
    let instructions = vec![
        ("Enter", Color::Green, action_text),
        (" • ", Color::Gray, ""),
        ("Tab", Color::Cyan, " Select Project"),
        (" • ", Color::Gray, ""),
        ("Esc", Color::Red, " Cancel"),
    ];

    let mut instruction_text = Vec::new();
    for (key, color, desc) in instructions {
        instruction_text.push(ratatui::text::Span::styled(
            key,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
        instruction_text.push(ratatui::text::Span::styled(desc, Style::default().fg(Color::Gray)));
    }

    let instructions_paragraph =
        Paragraph::new(ratatui::text::Line::from(instruction_text)).alignment(Alignment::Center);

    f.render_widget(instructions_paragraph, chunks[3]);
}

// Legacy wrapper functions for backward compatibility
pub fn render_task_creation_dialog(
    f: &mut Frame,
    area: Rect,
    icons: &IconService,
    input_buffer: &str,
    task_projects: &[&project::Model],
    selected_task_project_index: Option<usize>,
) {
    render_task_dialog(
        f,
        area,
        icons,
        input_buffer,
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
    task_projects: &[&project::Model],
    selected_task_project_index: Option<usize>,
) {
    render_task_dialog(
        f,
        area,
        icons,
        input_buffer,
        task_projects,
        selected_task_project_index,
        true, // is_editing = true for editing
    );
}
