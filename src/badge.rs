use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

/// Badge styles for different types of information
pub enum BadgeStyle {
    Primary,
    Success,
    Warning,
    Danger,
    Info,
    Secondary,
}

impl BadgeStyle {
    fn to_style(&self) -> Style {
        match self {
            BadgeStyle::Primary => Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            BadgeStyle::Success => Style::default()
                .bg(Color::Green)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            BadgeStyle::Warning => Style::default()
                .bg(Color::Yellow)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
            BadgeStyle::Danger => Style::default()
                .bg(Color::Red)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            BadgeStyle::Info => Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
            BadgeStyle::Secondary => Style::default()
                .bg(Color::Gray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        }
    }
}

/// Create a badge with text and style
pub fn create_badge(text: &str, style: BadgeStyle) -> Span<'static> {
    Span::styled(format!(" {} ", text), style.to_style())
}

/// Create a compact badge with icon and text
pub fn create_compact_badge(icon: &str, text: &str, style: BadgeStyle) -> Span<'static> {
    Span::styled(format!("{}{}", icon, text), style.to_style())
}

/// Create a priority badge based on Todoist priority levels
pub fn create_priority_badge(priority: i32) -> Option<Span<'static>> {
    match priority {
        4 => Some(create_badge("P1", BadgeStyle::Danger)),  // Urgent
        3 => Some(create_badge("P2", BadgeStyle::Warning)), // High
        2 => Some(create_badge("P3", BadgeStyle::Info)),    // Medium
        1 => None,                                          // Normal priority - no badge
        _ => None,
    }
}

/// Create badges for task metadata
pub fn create_task_badges(
    is_recurring: bool,
    has_deadline: bool,
    duration: Option<&str>,
    label_count: usize,
) -> Vec<Span<'static>> {
    let mut badges = Vec::new();

    if is_recurring {
        badges.push(create_compact_badge("🔄", "REC", BadgeStyle::Primary));
    }

    if has_deadline {
        badges.push(create_compact_badge("⏰", "DUE", BadgeStyle::Danger));
    }

    if let Some(duration) = duration {
        badges.push(create_badge(duration, BadgeStyle::Warning));
    }

    if label_count > 0 {
        badges.push(create_badge(&format!("{}L", label_count), BadgeStyle::Success));
    }

    badges
}

/// Create a status badge
pub fn create_status_badge(is_completed: bool) -> Span<'static> {
    if is_completed {
        create_badge("DONE", BadgeStyle::Success)
    } else {
        create_badge("TODO", BadgeStyle::Secondary)
    }
}

/// Create bordered badges using Unicode box drawing
pub fn create_bordered_badge(text: &str, color: Color) -> Span<'static> {
    Span::styled(
        format!("┤{}├", text),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )
}

/// Create pill-shaped badges
pub fn create_pill_badge(text: &str, style: BadgeStyle) -> Span<'static> {
    Span::styled(format!("◖{}◗", text), style.to_style())
}
