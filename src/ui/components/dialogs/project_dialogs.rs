use crate::icons::IconService;
use crate::ui::layout::LayoutManager;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render_project_creation_dialog(f: &mut Frame, area: Rect, icons: &IconService, input_buffer: &str) {
    let dialog_area = LayoutManager::centered_rect(60, 15, area);
    f.render_widget(Clear, dialog_area);

    let title = format!("{} Create New Project", icons.info());
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(Color::White));

    let content_text = format!("Project Name: {}", input_buffer);
    let content_paragraph = Paragraph::new(content_text)
        .block(Block::default().borders(Borders::ALL).title("Project Name"))
        .style(Style::default().fg(Color::White));

    let instructions = "Press Enter to create, Esc to cancel";
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

pub fn render_project_edit_dialog(f: &mut Frame, area: Rect, icons: &IconService, input_buffer: &str) {
    let dialog_area = LayoutManager::centered_rect(60, 15, area);
    f.render_widget(Clear, dialog_area);

    let title = format!("{} Edit Project", icons.info());
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(Color::White));

    let content_text = format!("Project Name: {}", input_buffer);
    let content_paragraph = Paragraph::new(content_text)
        .block(Block::default().borders(Borders::ALL).title("Project Name"))
        .style(Style::default().fg(Color::White));

    let instructions = "Press Enter to save, Esc to cancel";
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
