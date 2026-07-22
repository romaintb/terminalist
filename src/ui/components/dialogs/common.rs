use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

/// Creates a styled main dialog block
pub fn create_dialog_block<'a>(title: &'a str, theme_color: Color) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .title_style(Style::default().fg(theme_color).add_modifier(Modifier::BOLD))
        .style(Style::default().fg(theme_color))
}

/// Returns the horizontal scroll offset and on-screen cursor column for an input.
pub fn input_viewport(input_buffer: &str, cursor_position: usize, visible_width: u16) -> (u16, u16) {
    let prefix: String = input_buffer.chars().take(cursor_position).collect();
    let cursor_column = Line::from(prefix).width();
    let last_visible_column = usize::from(visible_width.saturating_sub(1));
    let scroll_offset = cursor_column.saturating_sub(last_visible_column);
    let visible_cursor_column = cursor_column.saturating_sub(scroll_offset);

    (
        u16::try_from(scroll_offset).unwrap_or(u16::MAX),
        u16::try_from(visible_cursor_column).unwrap_or(u16::MAX),
    )
}

/// Creates an input field that scrolls horizontally to keep the cursor visible.
pub fn create_input_paragraph<'a>(
    input_buffer: &'a str,
    cursor_position: usize,
    visible_width: u16,
    field_title: &str,
) -> Paragraph<'a> {
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(format!(" {} ", field_title))
        .title_style(Style::default().fg(Color::White))
        .style(Style::default().fg(Color::Gray));

    let (scroll_offset, _) = input_viewport(input_buffer, cursor_position, visible_width);

    Paragraph::new(input_buffer)
        .block(input_block)
        .style(Style::default().fg(Color::White))
        .scroll((0, scroll_offset))
}

/// Creates a selection field block (read-only display with title)
pub fn create_selection_paragraph(value: String, field_title: &str) -> Paragraph<'static> {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(format!(" {} ", field_title))
        .title_style(Style::default().fg(Color::White))
        .style(Style::default().fg(Color::Gray));

    Paragraph::new(value).block(block).style(Style::default().fg(Color::White))
}

/// Instruction shortcut definition: (key, color, description)
pub type InstructionShortcut = (&'static str, Color, &'static str);

/// Creates a paragraph with color-coded instruction shortcuts
pub fn create_instructions_paragraph<'a>(instructions: &[InstructionShortcut]) -> Paragraph<'a> {
    let mut instruction_text = Vec::new();
    for (key, color, desc) in instructions {
        instruction_text.push(Span::styled(
            *key,
            Style::default().fg(*color).add_modifier(Modifier::BOLD),
        ));
        instruction_text.push(Span::styled(*desc, Style::default().fg(Color::Gray)));
    }

    Paragraph::new(Line::from(instruction_text)).alignment(Alignment::Center)
}

/// Common instruction shortcuts used across dialogs
pub mod shortcuts {
    use super::*;

    pub const SEPARATOR: InstructionShortcut = (" • ", Color::Gray, "");
    pub const ESC_CANCEL: InstructionShortcut = ("Esc", Color::Red, " Cancel");
    pub const TAB_SELECT: InstructionShortcut = ("Tab", Color::Cyan, " Select");
}

#[cfg(test)]
mod tests {
    use super::input_viewport;

    #[test]
    fn long_input_scrolls_to_keep_cursor_inside_field() {
        assert_eq!(input_viewport("abcdefghijklmnopqrstuvwxyz", 26, 10), (17, 9));
    }

    #[test]
    fn short_input_does_not_scroll() {
        assert_eq!(input_viewport("task", 4, 10), (0, 4));
    }

    #[test]
    fn viewport_uses_terminal_column_width_for_wide_characters() {
        assert_eq!(input_viewport("ab🙂cd", 5, 5), (2, 4));
    }
}
