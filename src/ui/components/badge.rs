use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

use crate::todoist::LabelDisplay;

/// Create badges with parentheses for duration
#[must_use]
pub fn create_paren_badge(text: &str) -> Span<'static> {
    Span::styled(
        format!("({text})"),
        Style::default()
            .fg(Color::Black)
            .bg(Color::LightYellow)
            .add_modifier(Modifier::BOLD),
    )
}

/// Create a label badge with custom color
#[must_use]
pub fn create_label_badge(name: &str, color: &str) -> Span<'static> {
    let bg_color = crate::utils::color::convert_todoist_color(color);

    let style = Style::default().bg(bg_color).fg(Color::White).add_modifier(Modifier::BOLD);

    Span::styled(name.to_string(), style)
}

/// Create task badges optimized for terminal compatibility
#[must_use]
pub fn create_task_badges(
    is_recurring: bool,
    _has_deadline: bool,
    duration: Option<&str>,
    labels: &[LabelDisplay],
) -> Vec<Span<'static>> {
    let mut badges = Vec::new();

    if is_recurring {
        badges.push(Span::styled("🔄", Style::default()));
    }

    if let Some(duration) = duration {
        badges.push(create_paren_badge(duration));
    }

    // Add label badges
    for label in labels {
        badges.push(create_label_badge(&label.name, &label.color));
    }

    badges
}

/// Create priority badges with flag symbols
#[must_use]
pub fn create_priority_badge(priority: i32) -> Option<Span<'static>> {
    match priority {
        4 => Some(Span::styled(
            "⚑",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )), // P1 = red flag
        3 => Some(Span::styled(
            "⚑",
            Style::default().fg(Color::Rgb(255, 165, 0)).add_modifier(Modifier::BOLD),
        )), // P2 = orange flag
        2 => Some(Span::styled(
            "⚑",
            Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
        )), // P3 = blue flag
        1 => Some(Span::styled("⚐", Style::default().fg(Color::White))), // P4 = white flag (default color)
        _ => Some(Span::styled("⚐", Style::default().fg(Color::White))), // Unknown priority = P4 = white flag
    }
}
