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
        
        let help_content = r"
TERMINALIST - Todoist Terminal Client
====================================

NAVIGATION
----------
j/k         Navigate tasks (down/up)
J/K         Navigate projects (down/up)
Enter       Select project/task or confirm action
Esc         Cancel action or close dialogs

PROJECT MANAGEMENT
-----------------
A           Create new project
D           Delete selected project (with confirmation)

TASK MANAGEMENT
--------------
Space       Toggle task completion
a           Create new task
d           Delete task (with confirmation)

SYNC & DATA
-----------
r           Force sync with Todoist
Ctrl+C      Quit application

GENERAL CONTROLS
----------------
?           Toggle help panel
q           Quit application

HELP PANEL SCROLLING
--------------------
↑↓          Scroll help content up/down
Home        Jump to top of help
End         Jump to bottom of help

TASK STATUS INDICATORS
----------------------
🔳          Pending task
✅          Completed task
❌          Deleted task

Priority badges: [P0] (urgent), [P1] (high), [P2] (medium), [P3] (low), no badge (normal)

LAYOUT DETAILS
--------------
Left pane:  Projects list with selection
Right pane: Tasks for selected project
Bottom:     Status bar with shortcuts
Help:       Modal overlay with scrollable content

NOTES
-----
Tasks are ordered: pending, then completed, then deleted

Press 'Esc' or '?' to close this help panel
";
        
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
            format!("\n\n[Scroll: {scroll_percent}% - ↑↓ to navigate, Home/End for extremes]")
        } else {
            String::new()
        };
        
        let final_text = format!("{help_text}{scroll_indicator}");
        
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
