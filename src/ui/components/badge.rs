use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

use crate::todoist::LabelDisplay;

#[derive(Debug, Clone, Copy)]
/// Badge styles optimized for terminal compatibility
pub enum BadgeStyle {
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

impl BadgeStyle {
    fn to_style(self) -> Style {
        match self {
            // Use bright colors with proper backgrounds for better visibility
            BadgeStyle::Primary => Style::default()
                .fg(Color::Black)
                .bg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
            BadgeStyle::Success => Style::default()
                .fg(Color::Black)
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
            BadgeStyle::Warning => Style::default()
                .fg(Color::Black)
                .bg(Color::LightYellow)
                .add_modifier(Modifier::BOLD),
            BadgeStyle::Danger => Style::default()
                .fg(Color::White)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
            BadgeStyle::Info => Style::default()
                .fg(Color::Black)
                .bg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
            BadgeStyle::Secondary => Style::default()
                .fg(Color::White)
                .bg(Color::Gray)
                .add_modifier(Modifier::BOLD),
            BadgeStyle::Bordered | BadgeStyle::Bold => Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            BadgeStyle::Underlined => Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::UNDERLINED | Modifier::BOLD),
        }
    }
}

/// Create a terminal-optimized badge with text and style
#[must_use]
pub fn create_badge(text: &str, style: BadgeStyle) -> Span<'static> {
    Span::styled(format!(" {text} "), style.to_style())
}

/// Create badges with brackets (ASCII fallback)
#[must_use]
pub fn create_bracket_badge(text: &str, style: BadgeStyle) -> Span<'static> {
    Span::styled(format!("[{text}]"), style.to_style())
}

/// Create badges with parentheses
#[must_use]
pub fn create_paren_badge(text: &str, style: BadgeStyle) -> Span<'static> {
    Span::styled(format!("({text})"), style.to_style())
}

/// Create a label badge with custom color
#[must_use]
pub fn create_label_badge(name: &str, color: &str) -> Span<'static> {
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
        "navy" => Color::Rgb(0, 0, 128), // Navy
        _ => Color::Blue,                // Default fallback
    };

    let style = Style::default()
        .bg(bg_color)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    Span::styled(name.to_string(), style)
}

/// Create task badges optimized for terminal compatibility
#[must_use]
pub fn create_task_badges(
    is_recurring: bool,
    has_deadline: bool,
    duration: Option<&str>,
    labels: &[LabelDisplay],
) -> Vec<Span<'static>> {
    let mut badges = Vec::new();

    if is_recurring {
        badges.push(create_bracket_badge("REC", BadgeStyle::Primary));
    }

    if has_deadline {
        badges.push(create_bracket_badge("DUE", BadgeStyle::Danger));
    }

    if let Some(duration) = duration {
        badges.push(create_paren_badge(duration, BadgeStyle::Warning));
    }

    // Add label badges
    for label in labels {
        badges.push(create_label_badge(&label.name, &label.color));
    }

    badges
}

/// Create priority badges with better terminal support
#[must_use]
pub fn create_priority_badge(priority: i32) -> Option<Span<'static>> {
    match priority {
        4 => Some(create_badge("P1", BadgeStyle::Danger)), // P1 = red (highest priority)
        3 => Some(create_badge("P2", BadgeStyle::Warning)), // P2 = orange
        2 => Some(create_badge("P3", BadgeStyle::Info)),   // P3 = blue
        1 => Some(create_badge("P4", BadgeStyle::Secondary)), // P4 = white (no priority/default)
        _ => Some(create_badge("P4", BadgeStyle::Secondary)), // Unknown priority = P4 = white
    }
}
