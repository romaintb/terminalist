//! Layout management and calculations

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Manages layout calculations and constraints for the UI
pub struct LayoutManager;

impl LayoutManager {
    /// Calculate the main layout areas (projects+tasks on top, status bar below)
    #[must_use]
    pub fn main_layout(area: Rect) -> Vec<Rect> {
        let screen_width = area.width;
        let screen_height = area.height;

        // Top area: projects + tasks (all height except 1 line for status)
        let top_height = screen_height.saturating_sub(1);
        let top_area = Rect::new(0, 0, screen_width, top_height);

        // Bottom area: status bar (1 line height, full width)
        let status_area = Rect::new(0, top_height, screen_width, 1);

        vec![top_area, status_area]
    }

    /// Calculate the top pane layout (projects + tasks side by side)
    #[must_use]
    pub fn top_pane_layout(area: Rect) -> Vec<Rect> {
        let projects_width = std::cmp::min(area.width / 3, 30); // Projects: 1/3 of width, max 30
        let tasks_width = area.width.saturating_sub(projects_width); // Tasks: remaining width

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(projects_width),
                Constraint::Length(tasks_width),
            ])
            .split(area)
            .to_vec()
    }

    /// Calculate the right pane layout (tasks + status bar)
    #[must_use]
    pub fn right_pane_layout(area: Rect, status_height: u16) -> Vec<Rect> {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(status_height)])
            .split(area)
            .to_vec()
    }

    /// Calculate the status bar height based on content and terminal size
    #[must_use]
    pub fn calculate_status_height(
        _loading: bool,
        _syncing: bool,
        _completing_task: bool,
        _deleting_task: bool,
        _delete_confirmation: Option<&String>,
        _error_message: Option<&String>,
    ) -> u16 {
        // All states use 1 line for the status bar
        1
    }

    /// Calculate a centered rectangle within the given area
    #[must_use]
    pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }

    /// Calculate a centered rectangle with percentage width and fixed line height
    #[must_use]
    pub fn centered_rect_lines(percent_x: u16, height_lines: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(height_lines),
                Constraint::Min(0),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }

    /// Calculate help panel dimensions based on screen size
    #[must_use]
    pub fn help_panel_dimensions(screen_width: u16, screen_height: u16) -> (u16, u16) {
        let help_width = if screen_width < 80 { 70 } else { 80 };
        let help_height = if screen_height < 40 { 60 } else { 70 };
        (help_width, help_height)
    }

    /// Calculate sidebar constraints based on available width
    #[must_use]
    pub fn sidebar_constraints(sidebar_width: u16) -> (u16, u16) {
        let max_name_width = sidebar_width.saturating_sub(4); // Account for icon, space, and borders
        (sidebar_width, max_name_width)
    }

    /// Calculate task content constraints based on available width
    #[must_use]
    pub fn task_content_constraints(
        main_width: u16,
        icon_width: usize,
        badge_length: usize,
        badge_spacing: usize,
    ) -> usize {
        let task_box_width = main_width.saturating_sub(4); // Account for borders and padding
        (task_box_width as usize)
            .saturating_sub(icon_width)
            .saturating_sub(badge_length)
            .saturating_sub(badge_spacing)
    }
}
