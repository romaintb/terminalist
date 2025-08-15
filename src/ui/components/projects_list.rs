//! Projects list component

use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use super::super::app::App;

/// Projects list component
pub struct ProjectsList;

impl ProjectsList {
    /// Render the projects list
    pub fn render(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
        let (_sidebar_width, max_name_width) = super::super::layout::LayoutManager::sidebar_constraints(area.width);
        
        let project_items: Vec<ListItem> = app
            .projects
            .iter()
            .enumerate()
            .map(|(i, project)| {
                let icon = if project.is_favorite { "⭐" } else { "📁" };
                let style = if i == app.selected_project_index {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                // Truncate project name to fit sidebar
                let display_name = if project.name.len() > max_name_width as usize {
                    format!("{}…", &project.name[..max_name_width.saturating_sub(1) as usize])
                } else {
                    project.name.clone()
                };

                ListItem::new(Line::from(vec![
                    Span::styled(format!("{icon} "), style),
                    Span::styled(display_name, style),
                ]))
            })
            .collect();

        let projects_list = List::new(project_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("📁 Projects")
                    .title_alignment(Alignment::Center),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("→ ");

        f.render_stateful_widget(projects_list, area, &mut app.project_list_state.clone());
    }
}
