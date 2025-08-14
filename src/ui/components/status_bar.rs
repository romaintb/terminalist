//! Status bar component

use ratatui::{
    layout::Alignment,
    style::{Color, Style},
    widgets::{Block, Paragraph},
    Frame,
};

use super::super::app::App;

/// Status bar component
pub struct StatusBar;

impl StatusBar {
    /// Render the status bar
    pub fn render(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
        let status_text = if app.loading {
            "Loading local data...".to_string()
        } else if app.syncing {
            "🔄 Syncing with Todoist...".to_string()
        } else if app.completing_task {
            "🔄 Toggling task status...".to_string()
        } else if app.deleting_task {
            "🔄 Deleting task...".to_string()
        } else {
            // Show helpful shortcuts and status
            "Space: toggle • r: sync • d: delete • ?: help • q: quit".to_string()
        };

        let status_color = if app.syncing || app.completing_task {
            Color::Yellow
        } else if app.error_message.is_some() {
            Color::Red
        } else {
            Color::Gray
        };

        let status_bar = Paragraph::new(status_text)
            .block(Block::default())
            .alignment(Alignment::Center)
            .style(Style::default().fg(status_color));

        f.render_widget(status_bar, area);
    }
}
