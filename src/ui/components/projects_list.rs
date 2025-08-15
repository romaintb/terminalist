//! Projects list component

use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use super::super::app::App;
use crate::todoist::ProjectDisplay;

/// Projects list component
pub struct ProjectsList;

impl ProjectsList {
    /// Render the projects list
    pub fn render(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
        let (_sidebar_width, max_name_width) = super::super::layout::LayoutManager::sidebar_constraints(area.width);
        
        // Sort projects: favorites first within their own hierarchical level
        let mut sorted_projects: Vec<_> = app.projects.iter().enumerate().collect();
        
        // Helper function to get the root project ID (top-level parent)
        fn get_root_project_id(project: &ProjectDisplay, projects: &[ProjectDisplay]) -> String {
            let mut current = project;
            while let Some(parent_id) = &current.parent_id {
                if let Some(parent) = projects.iter().find(|p| p.id == *parent_id) {
                    current = parent;
                } else {
                    break;
                }
            }
            current.id.clone()
        }
        
        // Helper function to get the immediate parent ID
        fn get_immediate_parent_id(project: &ProjectDisplay) -> Option<String> {
            project.parent_id.clone()
        }
        
        sorted_projects.sort_by(|(_a_idx, a_project), (_b_idx, b_project)| {
            // First, sort by root project to keep tree structures together
            let a_root = get_root_project_id(a_project, &app.projects);
            let b_root = get_root_project_id(b_project, &app.projects);
            let root_cmp = a_root.cmp(&b_root);
            if root_cmp != std::cmp::Ordering::Equal {
                return root_cmp;
            }
            
            // Same root, now sort by immediate parent to keep siblings together
            let a_parent = get_immediate_parent_id(a_project);
            let b_parent = get_immediate_parent_id(b_project);
            let parent_cmp = a_parent.cmp(&b_parent);
            if parent_cmp != std::cmp::Ordering::Equal {
                return parent_cmp;
            }
            
            // Same immediate parent (siblings), sort favorites first, then by name
            match (a_project.is_favorite, b_project.is_favorite) {
                (true, false) => std::cmp::Ordering::Less,    // a (favorite) comes before b (non-favorite)
                (false, true) => std::cmp::Ordering::Greater, // a (non-favorite) comes after b (favorite)
                _ => a_project.name.cmp(&b_project.name),     // Same favorite status, sort by name
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

                // Calculate indentation based on tree depth
                let depth = calculate_tree_depth(project, &app.projects);
                let indent = "  ".repeat(depth);

                // Truncate project name to fit sidebar (accounting for indentation)
                let available_width = max_name_width.saturating_sub(indent.len() as u16);
                let display_name = if project.name.len() > available_width as usize {
                    format!("{}…", &project.name[..available_width.saturating_sub(1) as usize])
                } else {
                    project.name.clone()
                };

                ListItem::new(Line::from(vec![
                    Span::styled(indent, style),
                    Span::styled(format!("{icon} "), style),
                    Span::styled(display_name, style),
                ]))
            })
            .collect();

        // Helper function to calculate the actual tree depth of a project
        fn calculate_tree_depth(project: &ProjectDisplay, projects: &[ProjectDisplay]) -> usize {
            let mut depth = 0;
            let mut current = project;
            while let Some(parent_id) = &current.parent_id {
                if let Some(parent) = projects.iter().find(|p| p.id == *parent_id) {
                    depth += 1;
                    current = parent;
                } else {
                    break;
                }
            }
            depth
        }

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
