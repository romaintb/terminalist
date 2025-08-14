use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

/// Alternative badge styles that work better in terminals with limited color support
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
    fn to_style(&self) -> Style {
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
            TerminalBadgeStyle::Bordered => Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            TerminalBadgeStyle::Underlined => Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::UNDERLINED | Modifier::BOLD),
            TerminalBadgeStyle::Bold => Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        }
    }
}

/// Create a badge using reverse video (works in most terminals)
pub fn create_terminal_badge(text: &str, style: TerminalBadgeStyle) -> Span<'static> {
    Span::styled(format!(" {} ", text), style.to_style())
}

/// Create badges with Unicode box drawing (no color needed)
pub fn create_box_badge(text: &str) -> Span<'static> {
    Span::styled(
        format!("┌{}┐", text),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )
}

/// Create badges with brackets (ASCII fallback)
pub fn create_bracket_badge(text: &str, style: TerminalBadgeStyle) -> Span<'static> {
    Span::styled(format!("[{}]", text), style.to_style())
}

/// Create badges with parentheses
pub fn create_paren_badge(text: &str, style: TerminalBadgeStyle) -> Span<'static> {
    Span::styled(format!("({})", text), style.to_style())
}

/// Create simple text badges with modifiers only
pub fn create_text_badge(text: &str, style: TerminalBadgeStyle) -> Span<'static> {
    Span::styled(text.to_string(), style.to_style())
}

/// Create task badges optimized for terminal compatibility
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
            &format!("{}L", label_count),
            TerminalBadgeStyle::Success,
        ));
    }

    badges
}

/// Create priority badges with better terminal support
pub fn create_terminal_priority_badge(priority: i32) -> Option<Span<'static>> {
    match priority {
        4 => Some(create_terminal_badge("P0", TerminalBadgeStyle::Danger)), // Urgent
        3 => Some(create_terminal_badge("P1", TerminalBadgeStyle::Warning)), // High
        2 => Some(create_terminal_badge("P2", TerminalBadgeStyle::Info)),   // Medium
        1 => Some(create_terminal_badge("P3", TerminalBadgeStyle::Secondary)), // Low
        _ => None,                                                         // No priority
    }
}

/// Create alternative badge styles for testing
pub fn create_test_badges() -> Vec<(&'static str, Span<'static>)> {
    vec![
        (
            "Terminal Primary",
            create_terminal_badge("PRIMARY", TerminalBadgeStyle::Primary),
        ),
        (
            "Terminal Success",
            create_terminal_badge("SUCCESS", TerminalBadgeStyle::Success),
        ),
        (
            "Terminal Warning",
            create_terminal_badge("WARNING", TerminalBadgeStyle::Warning),
        ),
        (
            "Terminal Danger",
            create_terminal_badge("DANGER", TerminalBadgeStyle::Danger),
        ),
        ("Box Badge", create_box_badge("BOX")),
        (
            "Bracket Badge",
            create_bracket_badge("BRACKET", TerminalBadgeStyle::Info),
        ),
        (
            "Paren Badge",
            create_paren_badge("PAREN", TerminalBadgeStyle::Secondary),
        ),
        ("Text Bold", create_text_badge("BOLD", TerminalBadgeStyle::Bold)),
        (
            "Text Underlined",
            create_text_badge("UNDERLINE", TerminalBadgeStyle::Underlined),
        ),
    ]
}
