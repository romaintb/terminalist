use crate::icons::IconService;
use crate::todoist::ProjectDisplay;
use crate::ui::layout::LayoutManager;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render_task_creation_dialog(
    f: &mut Frame,
    area: Rect,
    icons: &IconService,
    input_buffer: &str,
    projects: &[ProjectDisplay],
    selected_project_index: usize,
) {
    let dialog_area = LayoutManager::centered_rect_lines(60, 10, area);
    f.render_widget(Clear, dialog_area);

    let title = format!("{} Create New Task", icons.info());
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(Color::White));

    let content_text = format!("Content: {}", input_buffer);
    let content_paragraph = Paragraph::new(content_text)
        .block(Block::default().borders(Borders::ALL).title("Task Content"))
        .style(Style::default().fg(Color::White));

    let project_name = if !projects.is_empty() {
        &projects[selected_project_index].name
    } else {
        "No projects available"
    };
    let project_text = format!("Project: {}", project_name);
    let project_paragraph = Paragraph::new(project_text)
        .block(Block::default().borders(Borders::ALL).title("Project"))
        .style(Style::default().fg(Color::White));

    let instructions = "Press Enter to create, Esc to cancel, Tab to change project";
    let instructions_paragraph = Paragraph::new(instructions)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);

    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(3),
            ratatui::layout::Constraint::Length(3),
            ratatui::layout::Constraint::Length(1),
        ])
        .split(dialog_area);

    f.render_widget(block, dialog_area);
    f.render_widget(content_paragraph, chunks[0]);
    f.render_widget(project_paragraph, chunks[1]);
    f.render_widget(instructions_paragraph, chunks[2]);
}

pub fn render_task_edit_dialog(f: &mut Frame, area: Rect, icons: &IconService, input_buffer: &str) {
    let dialog_area = LayoutManager::centered_rect_lines(60, 8, area);
    f.render_widget(Clear, dialog_area);

    let title = format!("{} Edit Task", icons.info());
    let content_text = format!("Content: {}", input_buffer);
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
