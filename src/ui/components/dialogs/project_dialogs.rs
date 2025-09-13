use crate::icons::IconService;
use crate::ui::layout::LayoutManager;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

pub fn render_project_creation_dialog(
    f: &mut Frame,
    area: Rect,
    _icons: &IconService,
    input_buffer: &str,
    root_projects: &[&crate::todoist::ProjectDisplay],
    selected_parent_index: Option<usize>,
) {
    let dialog_area = LayoutManager::centered_rect_lines(65, 12, area);
    f.render_widget(Clear, dialog_area);

    // Main dialog block with rounded borders and magenta theme
    let title = "New Project".to_string();
    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .title_style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
        .style(Style::default().fg(Color::Magenta));

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

    // Input field with visual cursor
    let cursor_char = "█";
    let input_display = format!("{}{}", input_buffer, cursor_char);

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Project Name ")
        .title_style(Style::default().fg(Color::White))
        .style(Style::default().fg(Color::Gray));

    let input_paragraph = Paragraph::new(input_display)
        .block(input_block)
        .style(Style::default().fg(Color::White));

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

    let parent_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Parent Project ")
        .title_style(Style::default().fg(Color::White))
        .style(Style::default().fg(Color::Gray));

    let parent_paragraph = Paragraph::new(parent_project_name)
        .block(parent_block)
        .style(Style::default().fg(Color::White));

    // Enhanced instructions with color-coded shortcuts
    let instructions = vec![
        ("Enter", Color::Green, " Create Project"),
        (" • ", Color::Gray, ""),
        ("Tab", Color::Cyan, " Select Parent"),
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

    // Render all components
    f.render_widget(main_block, dialog_area);
    f.render_widget(input_paragraph, chunks[0]);
    f.render_widget(parent_paragraph, chunks[1]);
    f.render_widget(instructions_paragraph, chunks[3]);
}

pub fn render_project_edit_dialog(f: &mut Frame, area: Rect, _icons: &IconService, input_buffer: &str) {
    let dialog_area = LayoutManager::centered_rect_lines(65, 9, area);
    f.render_widget(Clear, dialog_area);

    // Main dialog block with rounded borders and yellow theme for edit
    let title = "Edit Project".to_string();
    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .style(Style::default().fg(Color::Yellow));

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

    // Input field with visual cursor
    let cursor_char = "█";
    let input_display = format!("{}{}", input_buffer, cursor_char);

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Project Name ")
        .title_style(Style::default().fg(Color::White))
        .style(Style::default().fg(Color::Gray));

    let input_paragraph = Paragraph::new(input_display)
        .block(input_block)
        .style(Style::default().fg(Color::White));

    // Enhanced instructions with color-coded shortcuts
    let instructions = vec![
        ("Enter", Color::Green, " Save Changes"),
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

    // Render all components
    f.render_widget(main_block, dialog_area);
    f.render_widget(input_paragraph, chunks[0]);
    f.render_widget(instructions_paragraph, chunks[2]);
}
