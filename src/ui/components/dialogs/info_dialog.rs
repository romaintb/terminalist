//! Info dialog component for displaying success and informational messages

use ratatui::{
    layout::Alignment,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::super::super::app::App;
use super::super::super::layout::LayoutManager;

/// Info dialog component for success and informational messages
pub struct InfoDialog;

impl InfoDialog {
    /// Render the info dialog
    pub fn render(f: &mut Frame, app: &App) {
        if let Some(info_msg) = &app.info_message {
            let info_area = LayoutManager::centered_rect(60, 20, f.area());
            f.render_widget(Clear, info_area);
            let display_text = format!("{}\n\nPress Enter or Esc to dismiss", info_msg);
            let info_paragraph = Paragraph::new(display_text.as_str())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Success")
                        .title_alignment(Alignment::Center),
                )
                .style(Style::default().fg(Color::Green))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            f.render_widget(info_paragraph, info_area);
        }
    }
}
