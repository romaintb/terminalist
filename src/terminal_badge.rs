use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

/// Alternative badge styles that work better in terminals with limited color support
#[derive(Debug, Clone, Copy)]
pub enum TerminalBadgeStyle {
    Primary,
    Success,
    Warning,
    Danger,
    Info,
    Secondary,
    Bordered,
    Underlined,
    Bold,
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

/// Create task badges optimized for terminal compatibility
#[must_use]
pub fn create_terminal_task_badges(
    is_recurring: bool,
    has_deadline: bool,
    duration: Option<&str>,
    label_count: usize,
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

    if label_count > 0 {
        badges.push(create_bracket_badge(
            &format!("{label_count}L"),
            TerminalBadgeStyle::Success,
        ));
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
