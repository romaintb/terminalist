use ratatui::{prelude::*, style::Color};
use terminalist::terminal_badge::*;

fn main() {
    println!("🎨 Terminal-Optimized Badge Test\n");

    println!("These badges are designed to work better in terminals with limited color support:");
    println!("They use REVERSED text, brackets, and text modifiers instead of background colors.\n");

    // Test the alternative badge styles
    let test_badges = create_test_badges();

    for (name, badge) in test_badges {
        println!("  {:<18}: '{}'", name, badge.content);
    }

    println!("\nTask badges example:");
    let task_badges = create_terminal_task_badges(true, true, Some("2h"), 3);
    print!("  Task: Buy groceries ");
    for badge in task_badges {
        print!("{} ", badge.content);
    }

    if let Some(priority_badge) = create_terminal_priority_badge(4) {
        print!("{}", priority_badge.content);
    }
    println!();

    println!("\nPriority badges:");
    for priority in [4, 3, 2, 1] {
        if let Some(badge) = create_terminal_priority_badge(priority) {
            println!("  Priority {}: '{}'", 5 - priority, badge.content);
        } else {
            println!("  Priority {}: (no badge)", 5 - priority);
        }
    }

    println!("\n💡 These should be more visible in your terminal!");
    println!("   If you still see only white text, try running in a different terminal emulator.");
}
