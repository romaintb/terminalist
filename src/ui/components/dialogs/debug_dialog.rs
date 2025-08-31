//! Debug dialog component for displaying debug logs

use crate::ui::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

/// Debug dialog component for displaying debug logs
pub struct DebugDialog;

impl DebugDialog {
    /// Render the debug dialog
    pub fn render(frame: &mut Frame, app: &App, area: Rect) {
        // Calculate modal size - take 80% of the screen
        let modal_width = area.width.saturating_mul(8) / 10;
        let modal_height = area.height.saturating_mul(8) / 10;

        let modal_x = (area.width.saturating_sub(modal_width)) / 2;
        let modal_y = (area.height.saturating_sub(modal_height)) / 2;

        let modal_area = Rect {
            x: area.x + modal_x,
            y: area.y + modal_y,
            width: modal_width,
            height: modal_height,
        };

        // Clear the area behind the modal
        frame.render_widget(Clear, modal_area);

        // Create the modal block
        let block = Block::default()
            .title("Debug Logs")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black))
            .border_style(Style::default().fg(Color::Cyan));

        frame.render_widget(block, modal_area);

        // Calculate the inner area for content
        let inner_area = modal_area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        });

        // Create layout for header and content
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(0)])
            .split(inner_area);

        // Render header with instructions
        let header = Paragraph::new(Line::from(vec![
            Span::styled("Press ", Style::default().fg(Color::Gray)),
            Span::styled(
                "q",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to close, ", Style::default().fg(Color::Gray)),
            Span::styled(
                "↑/↓",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" or ", Style::default().fg(Color::Gray)),
            Span::styled(
                "j/k",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to scroll", Style::default().fg(Color::Gray)),
        ]))
        .alignment(Alignment::Center);

        frame.render_widget(header, chunks[0]);

        // Render debug logs
        let debug_logs = app.debug_logger.get_logs();
        if debug_logs.is_empty() {
            let no_logs = Paragraph::new("No debug logs available")
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);
            frame.render_widget(no_logs, chunks[1]);
        } else {
            // Create list items from debug logs
            let log_items: Vec<ListItem> = debug_logs
                .iter()
                .enumerate()
                .map(|(i, log)| {
                    let style = if i % 2 == 0 {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default().fg(Color::Gray)
                    };

                    // Parse timestamp and message
                    if let Some(bracket_end) = log.find("] ") {
                        let timestamp = &log[1..bracket_end];
                        let message = &log[bracket_end + 2..];

                        // Color code based on log content
                        let message_style = if message.contains("❌") || message.contains("Failed") {
                            Style::default().fg(Color::Red)
                        } else if message.contains("✅") || message.contains("Success") {
                            Style::default().fg(Color::Green)
                        } else if message.contains("⚠️") || message.contains("Warning") {
                            Style::default().fg(Color::Yellow)
                        } else if message.contains("🔄") || message.contains("Starting") {
                            Style::default().fg(Color::Cyan)
                        } else if message.contains("💾") || message.contains("Storing") {
                            Style::default().fg(Color::Blue)
                        } else if message.contains("📱") || message.contains("Loading") {
                            Style::default().fg(Color::Magenta)
                        } else {
                            style
                        };

                        ListItem::new(Line::from(vec![
                            Span::styled(format!("[{}] ", timestamp), Style::default().fg(Color::DarkGray)),
                            Span::styled(message, message_style),
                        ]))
                    } else {
                        ListItem::new(Line::from(Span::styled(log, style)))
                    }
                })
                .collect();

            // Calculate which logs to show based on scroll offset
            let visible_height = chunks[1].height as usize;
            let start_index = app
                .debug_scroll_offset
                .min(debug_logs.len().saturating_sub(1));
            let end_index = (start_index + visible_height).min(debug_logs.len());

            let visible_items = log_items
                .into_iter()
                .skip(start_index)
                .take(end_index - start_index)
                .collect::<Vec<_>>();

            let logs_list = List::new(visible_items).style(Style::default().fg(Color::White));

            frame.render_widget(logs_list, chunks[1]);

            // Show scroll indicator if needed
            if debug_logs.len() > visible_height {
                let scroll_info = format!("Showing {}-{} of {} logs", start_index + 1, end_index, debug_logs.len());

                let scroll_area = Rect {
                    x: chunks[1].x + chunks[1].width.saturating_sub(scroll_info.len() as u16 + 2),
                    y: chunks[1].y + chunks[1].height.saturating_sub(1),
                    width: scroll_info.len() as u16 + 2,
                    height: 1,
                };

                let scroll_indicator = Paragraph::new(scroll_info)
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(Alignment::Right);

                frame.render_widget(scroll_indicator, scroll_area);
            }
        }
    }
}
