//! Scrollbar helper utilities for components with scrollable content.
//!
//! This module provides reusable scrollbar functionality that can be shared
//! across multiple UI components to avoid code duplication and ensure
//! consistent scrollbar behavior and styling.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

/// Helper for managing scrollbar state and rendering for scrollable components.
///
/// This struct encapsulates all scrollbar-related functionality including:
/// - Determining when a scrollbar is needed
/// - Calculating layout areas (content + scrollbar)
/// - Managing scrollbar state
/// - Rendering the scrollbar widget
pub struct ScrollbarHelper {
    state: ScrollbarState,
}

impl Default for ScrollbarHelper {
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollbarHelper {
    /// Create a new scrollbar helper with default state.
    pub fn new() -> Self {
        Self {
            state: ScrollbarState::new(0),
        }
    }

    /// Update the scrollbar state with current content information.
    ///
    /// # Arguments
    /// * `total_items` - Total number of items in the scrollable content
    /// * `current_position` - Current selected/visible position (0-based index)
    /// * `viewport_height` - Optional viewport height for better scrollbar sizing
    pub fn update_state(&mut self, total_items: usize, current_position: usize, viewport_height: Option<usize>) {
        self.state = self.state.content_length(total_items).position(current_position);

        if let Some(height) = viewport_height {
            self.state = self.state.viewport_content_length(height);
        }
    }

    /// Check if a scrollbar is needed based on content size and available space.
    ///
    /// # Arguments
    /// * `total_items` - Total number of items in the content
    /// * `available_height` - Available height for rendering content (excluding borders)
    ///
    /// # Returns
    /// `true` if scrollbar is needed, `false` otherwise
    pub fn needs_scrollbar(total_items: usize, available_height: usize) -> bool {
        total_items > available_height
    }

    /// Calculate layout areas for content and scrollbar.
    ///
    /// Splits the given rectangle into separate areas for the main content
    /// and the scrollbar (if needed).
    ///
    /// # Arguments
    /// * `rect` - The total available rectangle
    /// * `total_items` - Total number of items to display
    ///
    /// # Returns
    /// A tuple of (content_area, optional_scrollbar_area)
    pub fn calculate_areas(rect: Rect, total_items: usize) -> (Rect, Option<Rect>) {
        let available_height = rect.height.saturating_sub(2) as usize; // Exclude borders
        let needs_scrollbar = Self::needs_scrollbar(total_items, available_height);

        if needs_scrollbar {
            let content_area = Rect {
                x: rect.x,
                y: rect.y,
                width: rect.width.saturating_sub(1), // Reserve 1 column for scrollbar
                height: rect.height,
            };
            let scrollbar_area = Rect {
                x: rect.x + rect.width.saturating_sub(1),
                y: rect.y + 1, // Start below top border
                width: 1,
                height: rect.height.saturating_sub(2), // Exclude top and bottom borders
            };
            (content_area, Some(scrollbar_area))
        } else {
            (rect, None)
        }
    }

    /// Render the scrollbar widget if a scrollbar area is provided.
    ///
    /// Uses consistent styling across all components:
    /// - Vertical orientation on the right side
    /// - Up/down arrow symbols at the ends
    /// - Vertical bar track symbol
    /// - Dark gray styling for unobtrusive appearance
    ///
    /// # Arguments
    /// * `f` - The frame to render to
    /// * `scrollbar_area` - The area to render the scrollbar in (if Some)
    pub fn render(&mut self, f: &mut Frame, scrollbar_area: Option<Rect>) {
        if let Some(area) = scrollbar_area {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"))
                .track_symbol(Some("│"))
                .thumb_symbol("█")
                .style(Style::default().fg(Color::DarkGray))
                .thumb_style(Style::default().fg(Color::DarkGray));

            f.render_stateful_widget(scrollbar, area, &mut self.state);
        }
    }

    /// Get a mutable reference to the internal scrollbar state.
    ///
    /// This can be useful for advanced scrollbar state manipulation
    /// that isn't covered by the helper methods.
    pub fn state_mut(&mut self) -> &mut ScrollbarState {
        &mut self.state
    }

    /// Get an immutable reference to the internal scrollbar state.
    pub fn state(&self) -> &ScrollbarState {
        &self.state
    }
}
