use ratatui::style::Color;

/// Convert Todoist color names to terminal colors
#[must_use]
pub fn convert_todoist_color(color: &str) -> Color {
    match color.to_lowercase().as_str() {
        "berry_red" => Color::Rgb(184, 37, 95),
        "red" => Color::Rgb(220, 76, 62),
        "orange" => Color::Rgb(199, 113, 0),
        "yellow" => Color::Rgb(178, 145, 4),
        "olive_green" => Color::Rgb(148, 156, 49),
        "lime_green" => Color::Rgb(101, 163, 58),
        "green" => Color::Rgb(54, 147, 7),
        "mint_green" => Color::Rgb(66, 163, 147),
        "teal" => Color::Rgb(20, 143, 173),
        "sky_blue" => Color::Rgb(49, 157, 192),
        "light_blue" => Color::Rgb(105, 136, 164),
        "blue" => Color::Rgb(65, 128, 255),
        "grape" => Color::Rgb(105, 46, 194),
        "violet" => Color::Rgb(202, 63, 238),
        "lavender" => Color::Rgb(164, 105, 140),
        "magenta" => Color::Rgb(224, 80, 149),
        "salmon" => Color::Rgb(201, 118, 111),
        "charcoal" => Color::Rgb(128, 128, 128),
        "grey" | "gray" => Color::Rgb(153, 153, 153),
        "taupe" => Color::Rgb(143, 122, 105),
        _ => Color::Rgb(65, 128, 255), // Default to blue
    }
}