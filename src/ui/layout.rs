//! Layout management and calculations

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Manages layout calculations and constraints for the UI
pub struct LayoutManager;

impl LayoutManager {
    /// Calculate the main layout with sidebar and main content area
    pub fn main_layout(area: Rect) -> Vec<Rect> {
        let screen_width = area.width;
        let sidebar_width = std::cmp::min(screen_width * 30 / 100, 25);
        let main_width = screen_width.saturating_sub(sidebar_width);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(sidebar_width),
                Constraint::Length(main_width),
            ])
            .split(area)
            .to_vec()
    }

    /// Calculate the right pane layout with content and status bar
    pub fn right_pane_layout(area: Rect, status_height: u16) -> Vec<Rect> {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(status_height)])
            .split(area)
            .to_vec()
    }

    /// Calculate the status bar height based on content and terminal size
    pub fn calculate_status_height(
        loading: bool,
        syncing: bool,
        completing_task: bool,
        deleting_task: bool,
        delete_confirmation: Option<&String>,
        error_message: Option<&String>,
    ) -> u16 {
        if loading || syncing || completing_task || deleting_task {
            1 // Single line for simple status messages
        } else if delete_confirmation.is_some() || error_message.is_some() {
            1 // Single line for confirmation/error dialogs too
        } else {
            // Normal state: just 1 line for the shortcuts
            1
        }
    }

    /// Create a centered rectangle using percentage of available space
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

    /// Calculate adaptive help panel dimensions based on terminal size
    pub fn help_panel_dimensions(screen_width: u16, screen_height: u16) -> (u16, u16) {
        let help_width = if screen_width < 80 { 70 } else { 80 };
        let help_height = if screen_height < 40 { 60 } else { 70 };
        (help_width, help_height)
    }

    /// Calculate sidebar constraints for project names
    pub fn sidebar_constraints(sidebar_width: u16) -> (u16, u16) {
        let max_name_width = sidebar_width.saturating_sub(4); // Account for icon, space, and borders
        (sidebar_width, max_name_width)
    }

    /// Calculate task content constraints based on available width
    pub fn task_content_constraints(
        main_width: u16,
        icon_width: usize,
        badge_length: usize,
        badge_spacing: usize,
    ) -> usize {
        let task_box_width = main_width.saturating_sub(4); // Account for borders and padding
        let max_content_len = (task_box_width as usize)
            .saturating_sub(icon_width)
            .saturating_sub(badge_length)
            .saturating_sub(badge_spacing);
        max_content_len
    }
}
