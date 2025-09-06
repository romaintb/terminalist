use crate::debug_logger::DebugLogger;
use crate::icons::IconService;
use crate::ui::layout::LayoutManager;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

pub fn render_delete_confirmation_dialog(f: &mut Frame, area: Rect, icons: &IconService, item_type: &str) {
    let dialog_area = LayoutManager::centered_rect_lines(50, 6, area);
    f.render_widget(Clear, dialog_area);

    let title = format!("{} Confirm Delete", icons.warning());
    let message = format!("Are you sure you want to delete this {}?", item_type);
    let instructions = "Press Enter to confirm, Esc to cancel";

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(Color::Red));

    let message_paragraph = Paragraph::new(message)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center);

    let instructions_paragraph = Paragraph::new(instructions)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);

    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(2),
            ratatui::layout::Constraint::Length(1),
        ])
        .split(dialog_area);

    f.render_widget(block, dialog_area);
    f.render_widget(message_paragraph, chunks[0]);
    f.render_widget(instructions_paragraph, chunks[1]);
}

pub fn render_info_dialog(
    f: &mut Frame,
    area: Rect,
    icons: &IconService,
    message: &str,
    scroll_offset: usize,
    scrollbar_state: &mut ScrollbarState,
) {
    let dialog_area = LayoutManager::centered_rect_lines(60, 10, area);
    f.render_widget(Clear, dialog_area);

    let title = format!("{} Info", icons.info());
    let instructions = "Press any key to continue • j/k to scroll if needed";

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(Color::Blue));

    let content_area = Rect::new(
        dialog_area.x + 1,
        dialog_area.y + 1,
        dialog_area.width.saturating_sub(2),
        dialog_area.height.saturating_sub(4),
    );

    let instructions_area = Rect::new(
        dialog_area.x + 1,
        dialog_area.y + dialog_area.height.saturating_sub(2),
        dialog_area.width.saturating_sub(2),
        1,
    );

    let lines: Vec<&str> = message.lines().collect();
    let total_lines = lines.len();
    let visible_height = content_area.height as usize;

    let message_text = if total_lines > visible_height {
        let max_scroll = total_lines.saturating_sub(visible_height);
        let clamped_offset = scroll_offset.min(max_scroll);

        *scrollbar_state = scrollbar_state
            .content_length(total_lines)
            .viewport_content_length(visible_height)
            .position(clamped_offset);

        let visible_lines: Vec<&str> = lines
            .iter()
            .skip(clamped_offset)
            .take(visible_height)
            .copied()
            .collect();
        visible_lines.join("\n")
    } else {
        message.to_string()
    };

    let message_paragraph = Paragraph::new(message_text)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left)
        .wrap(ratatui::widgets::Wrap { trim: true });

    let instructions_paragraph = Paragraph::new(instructions)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);

    f.render_widget(block, dialog_area);
    f.render_widget(message_paragraph, content_area);
    f.render_widget(instructions_paragraph, instructions_area);

    if total_lines > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .track_symbol(Some("│"))
            .thumb_symbol("▐")
            .style(Style::default().fg(Color::Gray))
            .thumb_style(Style::default().fg(Color::White));

        f.render_stateful_widget(scrollbar, content_area, scrollbar_state);
    }
}

pub fn render_error_dialog(
    f: &mut Frame,
    area: Rect,
    icons: &IconService,
    message: &str,
    scroll_offset: usize,
    scrollbar_state: &mut ScrollbarState,
) {
    let dialog_area = LayoutManager::centered_rect_lines(70, 12, area);
    f.render_widget(Clear, dialog_area);

    let title = format!("{} Error", icons.warning());
    let instructions = "Press any key to continue • j/k to scroll if needed";

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(Color::Red));

    let content_area = Rect::new(
        dialog_area.x + 1,
        dialog_area.y + 1,
        dialog_area.width.saturating_sub(2),
        dialog_area.height.saturating_sub(4),
    );

    let instructions_area = Rect::new(
        dialog_area.x + 1,
        dialog_area.y + dialog_area.height.saturating_sub(2),
        dialog_area.width.saturating_sub(2),
        1,
    );

    let lines: Vec<&str> = message.lines().collect();
    let total_lines = lines.len();
    let visible_height = content_area.height as usize;

    let message_text = if total_lines > visible_height {
        let max_scroll = total_lines.saturating_sub(visible_height);
        let clamped_offset = scroll_offset.min(max_scroll);

        *scrollbar_state = scrollbar_state
            .content_length(total_lines)
            .viewport_content_length(visible_height)
            .position(clamped_offset);

        let visible_lines: Vec<&str> = lines
            .iter()
            .skip(clamped_offset)
            .take(visible_height)
            .copied()
            .collect();
        visible_lines.join("\n")
    } else {
        message.to_string()
    };

    let message_paragraph = Paragraph::new(message_text)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left)
        .wrap(ratatui::widgets::Wrap { trim: true });

    let instructions_paragraph = Paragraph::new(instructions)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);

    f.render_widget(block, dialog_area);
    f.render_widget(message_paragraph, content_area);
    f.render_widget(instructions_paragraph, instructions_area);

    if total_lines > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .track_symbol(Some("│"))
            .thumb_symbol("▐")
            .style(Style::default().fg(Color::Gray))
            .thumb_style(Style::default().fg(Color::White));

        f.render_stateful_widget(scrollbar, content_area, scrollbar_state);
    }
}

pub fn render_help_dialog(f: &mut Frame, area: Rect, scroll_offset: usize, scrollbar_state: &mut ScrollbarState) {
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
e           Edit selected task
d           Delete task (with confirmation)
t           Set task due date to today
T           Set task due date to tomorrow
w           Set task due date to next week (Monday)
W           Set task due date to next week end (Saturday)

SYNC & DATA
-----------
r           Force sync with Todoist
Ctrl+C      Quit application

GENERAL CONTROLS
----------------
?           Toggle help panel
h           Toggle help panel
q           Quit application
i           Change icon theme

HELP PANEL SCROLLING
--------------------
j/k         Scroll help content down/up
↑↓          Scroll help content up/down
PageUp/Down Page through help content
Home        Jump to top of help
End         Jump to bottom of help

TASK STATUS INDICATORS
----------------------
🔳          Pending task
✅          Completed task
❌          Deleted task

Priority badges: [P1] (red, highest), [P2] (orange), [P3] (blue), [P4] (white, default)

LAYOUT DETAILS
--------------
Left pane:  Projects list with selection
Right pane: Tasks for selected project
Bottom:     Status bar with shortcuts
Help:       Modal overlay with scrollable content

NOTES
-----
Tasks are ordered: pending, then completed, then deleted

Press 'Esc', '?' or 'h' to close this help panel
";

    let help_area = LayoutManager::centered_rect(90, 90, area);
    f.render_widget(Clear, help_area);

    let margin_x = 2;
    let margin_y = 1;
    let help_content_area = Rect::new(
        help_area.x + margin_x,
        help_area.y + margin_y,
        help_area.width.saturating_sub(margin_x * 2),
        help_area.height.saturating_sub(margin_y * 2),
    );

    let lines: Vec<&str> = help_content.lines().collect();
    let total_lines = lines.len();
    let visible_height = help_content_area.height.saturating_sub(2) as usize;

    let max_scroll = total_lines.saturating_sub(visible_height);
    let clamped_offset = scroll_offset.min(max_scroll);

    *scrollbar_state = scrollbar_state
        .content_length(total_lines)
        .viewport_content_length(visible_height)
        .position(clamped_offset);

    let visible_lines: Vec<&str> = lines
        .iter()
        .skip(clamped_offset)
        .take(visible_height)
        .copied()
        .collect();

    let help_text = visible_lines.join("\n");

    let help_paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("📖 Help - Press 'Esc', '?' or 'h' to close")
                .title_alignment(Alignment::Center),
        )
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left);

    f.render_widget(help_paragraph, help_content_area);

    if total_lines > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .track_symbol(Some("│"))
            .thumb_symbol("▐")
            .style(Style::default().fg(Color::Gray))
            .thumb_style(Style::default().fg(Color::White));

        f.render_stateful_widget(scrollbar, help_content_area, scrollbar_state);
    }
}

pub fn render_logs_dialog(
    f: &mut Frame,
    area: Rect,
    debug_logger: &Option<DebugLogger>,
    scroll_offset: usize,
    scrollbar_state: &mut ScrollbarState,
) {
    let logs_area = LayoutManager::centered_rect(90, 90, area);
    f.render_widget(Clear, logs_area);

    let margin_x = 2;
    let margin_y = 1;
    let logs_content_area = Rect::new(
        logs_area.x + margin_x,
        logs_area.y + margin_y,
        logs_area.width.saturating_sub(margin_x * 2),
        logs_area.height.saturating_sub(margin_y * 2),
    );

    let logs = if let Some(ref logger) = debug_logger {
        logger.get_logs()
    } else {
        vec!["No debug logger available".to_string()]
    };

    let logs_content = if logs.is_empty() {
        "No debug logs available".to_string()
    } else {
        logs.join("\n")
    };

    let lines: Vec<&str> = logs_content.lines().collect();
    let total_lines = lines.len();
    let visible_height = logs_content_area.height.saturating_sub(2) as usize;

    let max_scroll = total_lines.saturating_sub(visible_height);
    let clamped_offset = scroll_offset.min(max_scroll);

    *scrollbar_state = scrollbar_state
        .content_length(total_lines)
        .viewport_content_length(visible_height)
        .position(clamped_offset);

    let visible_lines: Vec<&str> = lines
        .iter()
        .skip(clamped_offset)
        .take(visible_height)
        .copied()
        .collect();

    let logs_text = visible_lines.join("\n");

    let logs_paragraph = Paragraph::new(logs_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("🔍 Debug Logs - Press 'Esc', 'G' or 'q' to close")
                .title_alignment(Alignment::Center),
        )
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left);

    f.render_widget(logs_paragraph, logs_content_area);

    if total_lines > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .track_symbol(Some("│"))
            .thumb_symbol("▐")
            .style(Style::default().fg(Color::Gray))
            .thumb_style(Style::default().fg(Color::White));

        f.render_stateful_widget(scrollbar, logs_content_area, scrollbar_state);
    }
}
