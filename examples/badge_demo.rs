use ratatui::{prelude::*, style::Color};
use terminalist::badge::*;

/// Demo showing different badge styles in ratatui
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example of different badge styles you can create
    let badge_examples = vec![
        // Bootstrap-style badges with background colors
        ("Primary", create_badge("NEW", BadgeStyle::Primary)),
        ("Success", create_badge("DONE", BadgeStyle::Success)),
        ("Warning", create_badge("DUE", BadgeStyle::Warning)),
        ("Danger", create_badge("URGENT", BadgeStyle::Danger)),
        ("Info", create_badge("INFO", BadgeStyle::Info)),
        ("Secondary", create_badge("TODO", BadgeStyle::Secondary)),
        // Compact badges with icons
        ("Recurring", create_compact_badge("🔄", "REC", BadgeStyle::Primary)),
        ("Deadline", create_compact_badge("⏰", "DUE", BadgeStyle::Danger)),
        ("Duration", create_badge("2h", BadgeStyle::Warning)),
        ("Labels", create_badge("5L", BadgeStyle::Success)),
        // Priority badges
        ("Priority 1", create_priority_badge(4).unwrap_or(Span::raw("None"))),
        ("Priority 2", create_priority_badge(3).unwrap_or(Span::raw("None"))),
        ("Priority 3", create_priority_badge(2).unwrap_or(Span::raw("None"))),
        // Bordered badges
        ("Bordered", create_bordered_badge("IMPORTANT", Color::Red)),
        // Pill-shaped badges
        ("Pill", create_pill_badge("FEATURE", BadgeStyle::Info)),
    ];

    println!("Badge Demo - Different styles available in ratatui:");
    for (name, badge) in &badge_examples {
        println!("  {:<12}: {}", name, badge.content);
    }

    println!("\nExample task with multiple badges:");
    let task_badges = create_task_badges(true, true, Some("2h"), 3);
    let priority_badge = create_priority_badge(4);

    print!("Task: Buy groceries ");
    for badge in task_badges {
        print!("{} ", badge.content);
    }
    if let Some(p_badge) = priority_badge {
        print!("{}", p_badge.content);
    }
    println!();

    Ok(())
}
