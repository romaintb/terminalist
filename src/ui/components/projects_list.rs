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
        
        // Sort projects: favorites first, then non-favorites, maintaining original order within each group
        let mut sorted_projects: Vec<_> = app.projects.iter().enumerate().collect();
        sorted_projects.sort_by(|(a_idx, a_project), (b_idx, b_project)| {
            match (a_project.is_favorite, b_project.is_favorite) {
                (true, false) => std::cmp::Ordering::Less,    // a (favorite) comes before b (non-favorite)
                (false, true) => std::cmp::Ordering::Greater, // a (non-favorite) comes after b (favorite)
                _ => {
                    // Same favorite status, sort by name, then by original index for stability
                    let name_cmp = a_project.name.cmp(&b_project.name);
                    if name_cmp == std::cmp::Ordering::Equal {
                        a_idx.cmp(b_idx)
                    } else {
                        name_cmp
                    }
                }
            }
        });
        
        let project_items: Vec<ListItem> = sorted_projects
            .iter()
            .map(|(original_index, project)| {
                let icon = if project.is_favorite { "⭐" } else { "📁" };
                let style = if *original_index == app.selected_project_index {
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
