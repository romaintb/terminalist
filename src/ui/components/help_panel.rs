//! Help panel component

use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::super::app::App;
use super::super::layout::LayoutManager;

/// Help panel component
pub struct HelpPanel;

impl HelpPanel {
    /// Render the help panel
    pub fn render(f: &mut Frame, app: &mut App) {
        // Adaptive help panel size based on terminal size
        let screen_width = f.size().width;
        let screen_height = f.size().height;
        
        let (help_width, help_height) = LayoutManager::help_panel_dimensions(screen_width, screen_height);
        
        let help_area = LayoutManager::centered_rect(help_width, help_height, f.size());
        f.render_widget(Clear, help_area);
        
        let help_content = r#"
TERMINALIST - Todoist Terminal Client
====================================

NAVIGATION & FOCUS
==================
  ← →          Navigate between projects (left sidebar)
  ↑ ↓          Navigate between tasks (main pane)
  Tab          Focus next pane
  Shift+Tab    Focus previous pane
  h/l          Alternative navigation keys (vim-style)

TASK MANAGEMENT
===============
  Space/Enter  Toggle task completion (complete ↔ reopen)
  d            Delete selected task (with confirmation, cannot redelete)
  r            Refresh/sync with Todoist API
  n            Create new task (coming soon)
  e            Edit selected task (coming soon)

PROJECT MANAGEMENT
==================
  p            Create new project (coming soon)
  e            Edit project (coming soon)
  f            Toggle project favorite status (coming soon)

SYNC & DATA
===========
  r            Force refresh from Todoist
  Auto-sync    Background sync every 5 minutes
  Offline      Browse cached data when offline
  Local DB     SQLite storage in ~/.local/share/terminalist/

GENERAL CONTROLS
================
  ?            Toggle this help panel
  q            Quit application gracefully
  Ctrl+C      Force quit (emergency exit)

HELP PANEL SCROLLING
====================
  ↑/k         Scroll up one line
  ↓/j         Scroll down one line
  PageUp      Scroll up 10 lines
  PageDown    Scroll down 10 lines
  Home        Jump to top
  End         Jump to bottom
  Esc/?       Close help panel

STATUS INDICATORS & BADGES
==========================
  ✅           Completed task (dimmed, green)
  🔳           Pending task (normal, white)
  ❌           Deleted task (red, strikethrough)

  Task Ordering:
  - Pending tasks appear first
  - Completed tasks appear second
  - Deleted tasks appear last
  - Deleted tasks cannot be redeleted
  
  Priority Levels:
  [P0]         P1 - Urgent (highest) - Red background
  [P1]         P2 - High - Yellow background
  [P2]         P3 - Medium - Cyan background  
  [P3]         P4 - Low - Gray background
  (no badge)   P4 - Normal (lowest priority)
  
  Task Metadata:
  🔄REC       Recurring task (blue badge)
  ⏰DUE       Task with deadline (red badge)
  (2h)        Duration estimate (yellow badge)
  (30m)       Duration in minutes
  (3L)        Number of labels (green badge)

INTERFACE LAYOUT
================
  Left Sidebar (30% width, max 25 chars):
    - Project list with favorites (⭐)
    - Responsive width adaptation
    - Long names truncated with ellipsis (...)
  
  Main Pane (70% width):
    - Tasks for selected project
    - Rich metadata display
    - Interactive selection
  
  Status Bar (bottom):
    - Current project name
    - Task counts (pending/completed)
    - Sync status indicators
    - Keyboard shortcut hints

SYNC MECHANISM
==============
  Local Storage: SQLite database for instant loading
  Smart Sync: Background refresh every 5 minutes
  Manual Sync: Press 'r' for immediate refresh
  Offline Support: Browse cached data when disconnected
  Error Handling: Graceful fallback for network issues

PERFORMANCE FEATURES
====================
  Instant startup from local cache
  Background sync without blocking UI
  Efficient memory usage
  Responsive keyboard navigation
  Adaptive terminal size handling

TROUBLESHOOTING
===============
  No tasks showing: Press 'r' to sync
  Can't navigate: Check if project is selected
  Sync errors: Check API token and network
  Display issues: Resize terminal window
  Performance: Wait for background sync

For more information, visit the project repository
or check the README.md file for detailed setup instructions.

Press 'Esc' or '?' to close this help panel
        "#;
        
        // Apply scroll offset to the content
        let lines: Vec<&str> = help_content.lines().collect();
        let total_lines = lines.len();
        let visible_height = help_height.saturating_sub(2) as usize; // Account for borders
        
        // Clamp scroll offset to valid range
        let max_scroll = total_lines.saturating_sub(visible_height);
        let scroll_offset = app.help_scroll_offset.min(max_scroll);
        
        // Extract visible portion of content
        let visible_lines: Vec<&str> = lines
            .iter()
            .skip(scroll_offset)
            .take(visible_height)
            .copied()
            .collect();
        
        let help_text = visible_lines.join("\n");
        
        // Add scroll indicator if content is scrollable
        let scroll_indicator = if total_lines > visible_height {
            let scroll_percent = if max_scroll > 0 {
                (scroll_offset * 100) / max_scroll
            } else {
                0
            };
            format!("\n\n[Scroll: {}% - ↑↓ to navigate, Home/End for extremes]", scroll_percent)
        } else {
            String::new()
        };
        
        let final_text = format!("{}{}", help_text, scroll_indicator);
        
        let help_paragraph = Paragraph::new(final_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("❓ HELP PANEL (Modal) - {}/{} lines", scroll_offset + 1, total_lines))
                    .title_alignment(Alignment::Center)
                    .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            )
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });
        f.render_widget(help_paragraph, help_area);
    }
}
