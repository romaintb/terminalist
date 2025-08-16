use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

use crate::todoist::LabelDisplay;

#[derive(Debug, Clone, Copy)]
/// Badge styles optimized for terminal compatibility
pub enum TerminalBadgeStyle {
    Primary,
    Success,
    Warning,
    Danger,
    Info,
    Secondary,
    Bordered,
    Bold,
    Underlined,
}

impl TerminalBadgeStyle {
    fn to_style(self) -> Style {
        match self {
            // Use bright colors with proper backgrounds for better visibility
            TerminalBadgeStyle::Primary => Style::default()
                .fg(Color::Black)
                .bg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
            TerminalBadgeStyle::Success => Style::default()
                .fg(Color::Black)
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
            TerminalBadgeStyle::Warning => Style::default()
                .fg(Color::Black)
                .bg(Color::LightYellow)
                .add_modifier(Modifier::BOLD),
            TerminalBadgeStyle::Danger => Style::default()
                .fg(Color::White)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
            TerminalBadgeStyle::Info => Style::default()
                .fg(Color::Black)
                .bg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
            TerminalBadgeStyle::Secondary => Style::default()
                .fg(Color::White)
                .bg(Color::Gray)
                .add_modifier(Modifier::BOLD),
            TerminalBadgeStyle::Bordered | TerminalBadgeStyle::Bold => Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            TerminalBadgeStyle::Underlined => Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::UNDERLINED | Modifier::BOLD),
        }
    }
}

/// Create a terminal-optimized badge with text and style
#[must_use]
pub fn create_terminal_badge(text: &str, style: TerminalBadgeStyle) -> Span<'static> {
    Span::styled(format!(" {text} "), style.to_style())
}

/// Create badges with brackets (ASCII fallback)
#[must_use]
pub fn create_bracket_badge(text: &str, style: TerminalBadgeStyle) -> Span<'static> {
    Span::styled(format!("[{text}]"), style.to_style())
}

/// Create badges with parentheses
#[must_use]
pub fn create_paren_badge(text: &str, style: TerminalBadgeStyle) -> Span<'static> {
    Span::styled(format!("({text})"), style.to_style())
}

/// Create a label badge with custom color
#[must_use]
pub fn create_terminal_label_badge(name: &str, color: &str) -> Span<'static> {
    // Convert Todoist color names to terminal colors
    let bg_color = match color.to_lowercase().as_str() {
        "red" => Color::Red,
        "orange" => Color::Rgb(255, 165, 0), // Orange
        "yellow" => Color::Yellow,
        "green" => Color::Green,
        "blue" => Color::Blue,
        "purple" => Color::Magenta,
        "pink" => Color::Rgb(255, 192, 203), // Pink
        "brown" => Color::Rgb(139, 69, 19),  // Brown
        "charcoal" => Color::DarkGray,
        "gray" => Color::Gray,
        "silver" => Color::White, // Changed from LightGray which doesn't exist
        "teal" => Color::Cyan,
        "navy" => Color::Rgb(0, 0, 128),    // Navy
        _ => Color::Blue, // Default fallback
    };

    let style = Style::default()
        .bg(bg_color)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    Span::styled(format!(" {name} "), style)
}

/// Create task badges optimized for terminal compatibility
#[must_use]
pub fn create_terminal_task_badges(
    is_recurring: bool,
    has_deadline: bool,
    duration: Option<&str>,
    labels: &[LabelDisplay],
) -> Vec<Span<'static>> {
    let mut badges = Vec::new();

    if is_recurring {
        badges.push(create_bracket_badge("REC", TerminalBadgeStyle::Primary));
    }

    if has_deadline {
        badges.push(create_bracket_badge("DUE", TerminalBadgeStyle::Danger));
    }

    if let Some(duration) = duration {
        badges.push(create_paren_badge(duration, TerminalBadgeStyle::Warning));
    }

    // Add label badges
    for label in labels {
        badges.push(create_terminal_label_badge(&label.name, &label.color));
    }

    badges
}

/// Create priority badges with better terminal support
#[must_use]
pub fn create_terminal_priority_badge(priority: i32) -> Option<Span<'static>> {
    match priority {
        4 => Some(create_terminal_badge("P0", TerminalBadgeStyle::Danger)), // Urgent
        3 => Some(create_terminal_badge("P1", TerminalBadgeStyle::Warning)), // High
        2 => Some(create_terminal_badge("P2", TerminalBadgeStyle::Info)),   // Medium
        1 => Some(create_terminal_badge("P3", TerminalBadgeStyle::Secondary)), // Low
        _ => None,                                                         // No priority
    }
}
