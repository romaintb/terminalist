//! Simple modal dialog indicating syncing/fetching state

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::ui::app::App;

pub struct SyncingDialog;

impl SyncingDialog {
    pub fn render(f: &mut Frame, app: &App) {
        let area = Self::centered_rect(50, 25, f.area());

        let title = if app.loading {
            "Loading local data"
        } else {
            "Syncing with Todoist"
        };
        let spinner = "⟳"; // simple indicator
        let lines = vec![
            Line::from(Span::styled(
                format!("{spinner} {title}..."),
                Style::default().fg(Color::Yellow),
            )),
            Line::from(Span::raw("Press q to quit")),
        ];

        let paragraph = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Please wait"))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White));

        // Clear the area first to draw a modal
        f.render_widget(Clear, area);
        f.render_widget(paragraph, area);
    }

    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Percentage((100 - percent_y) / 2),
                    Constraint::Percentage(percent_y),
                    Constraint::Percentage((100 - percent_y) / 2),
                ]
                .as_ref(),
            )
            .split(r);

        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage((100 - percent_x) / 2),
                    Constraint::Percentage(percent_x),
                    Constraint::Percentage((100 - percent_x) / 2),
                ]
                .as_ref(),
            )
            .split(popup_layout[1]);

        horizontal[1]
    }
}
