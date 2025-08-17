//! Error dialog component

use ratatui::{
    layout::Alignment,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::super::super::app::App;
use super::super::super::layout::LayoutManager;

/// Error dialog component
pub struct ErrorDialog;

impl ErrorDialog {
    /// Render the error dialog
    pub fn render(f: &mut Frame, app: &App) {
        if let Some(error_msg) = &app.error_message {
            let error_area = LayoutManager::centered_rect(60, 20, f.area());
            f.render_widget(Clear, error_area);
            let error_paragraph = Paragraph::new(error_msg.as_str())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Error")
                        .title_alignment(Alignment::Center),
                )
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            f.render_widget(error_paragraph, error_area);
        }
    }
}
