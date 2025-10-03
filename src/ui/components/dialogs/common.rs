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

/// Creates an input field block with a visual cursor
pub fn create_input_paragraph<'a>(input_buffer: &'a str, field_title: &str) -> Paragraph<'a> {
    let cursor_char = "█";
    let input_display = format!("{}{}", input_buffer, cursor_char);

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(format!(" {} ", field_title))
        .title_style(Style::default().fg(Color::White))
        .style(Style::default().fg(Color::Gray));

    Paragraph::new(input_display)
        .block(input_block)
        .style(Style::default().fg(Color::White))
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
