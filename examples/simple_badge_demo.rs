use terminalist::badge::*;

fn main() {
    println!("🎨 Badge Demo - Bootstrap-style badges in ratatui!\n");

    // Create different badge styles
    let primary = create_badge("PRIMARY", BadgeStyle::Primary);
    let success = create_badge("SUCCESS", BadgeStyle::Success);
    let warning = create_badge("WARNING", BadgeStyle::Warning);
    let danger = create_badge("DANGER", BadgeStyle::Danger);

    println!("Basic badges:");
    println!("  Primary: {}", primary.content);
    println!("  Success: {}", success.content);
    println!("  Warning: {}", warning.content);
    println!("  Danger:  {}", danger.content);

    // Create compact badges with icons
    let recurring = create_compact_badge("🔄", "REC", BadgeStyle::Primary);
    let deadline = create_compact_badge("⏰", "DUE", BadgeStyle::Danger);

    println!("\nCompact badges:");
    println!("  Recurring: {}", recurring.content);
    println!("  Deadline:  {}", deadline.content);

    // Create priority badges
    println!("\nPriority badges:");
    if let Some(p1) = create_priority_badge(4) {
        println!("  Priority 1: {}", p1.content);
    }
    if let Some(p2) = create_priority_badge(3) {
        println!("  Priority 2: {}", p2.content);
    }
    if let Some(p3) = create_priority_badge(2) {
        println!("  Priority 3: {}", p3.content);
    }

    // Create task badges
    let task_badges = create_task_badges(true, true, Some("2h"), 3);
    println!("\nTask badges for a sample task:");
    print!("  Task: Buy groceries ");
    for badge in task_badges {
        print!("{} ", badge.content);
    }
    println!();

    println!("\n✨ These badges provide Bootstrap-like styling in your terminal UI!");
}
