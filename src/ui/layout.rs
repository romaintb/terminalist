//! Layout management and calculations

use ratatui::layout::{Constraint, Layout, Rect};

/// Manages layout calculations and constraints for the UI
pub struct LayoutManager;

impl LayoutManager {
    /// Calculate a centered rectangle within the given area
    #[must_use]
    pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::vertical([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

        Layout::horizontal([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
    }

    /// Calculate a centered rectangle with percentage width and fixed line height
    #[must_use]
    pub fn centered_rect_lines(percent_x: u16, height_lines: u16, r: Rect) -> Rect {
        let popup_layout = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(height_lines),
            Constraint::Min(0),
        ])
        .split(r);

        Layout::horizontal([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
    }
}
