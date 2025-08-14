use ratatui::{prelude::*, style::Color};

fn main() {
    println!("🎨 Terminal Color Support Test\n");

    // Test basic colors
    println!("Basic ANSI Colors:");
    let colors = vec![
        ("Black", Color::Black),
        ("Red", Color::Red),
        ("Green", Color::Green),
        ("Yellow", Color::Yellow),
        ("Blue", Color::Blue),
        ("Magenta", Color::Magenta),
        ("Cyan", Color::Cyan),
        ("White", Color::White),
        ("Gray", Color::Gray),
        ("DarkGray", Color::DarkGray),
        ("LightRed", Color::LightRed),
        ("LightGreen", Color::LightGreen),
        ("LightYellow", Color::LightYellow),
        ("LightBlue", Color::LightBlue),
        ("LightMagenta", Color::LightMagenta),
        ("LightCyan", Color::LightCyan),
    ];

    for (name, color) in colors {
        // Create a span with background color
        let span = Span::styled(format!(" {} ", name), Style::default().bg(color).fg(Color::White));
        println!("  {}: '{}'", name, span.content);
    }

    println!("\nEnvironment Info:");
    println!("  TERM: {:?}", std::env::var("TERM"));
    println!("  COLORTERM: {:?}", std::env::var("COLORTERM"));
    println!("  TERM_PROGRAM: {:?}", std::env::var("TERM_PROGRAM"));

    println!("\nIf you see background colors above, your terminal supports them!");
    println!("If not, we'll need to use alternative styling approaches.");
}
