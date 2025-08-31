//! Tasks list component

use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use super::super::app::App;
use crate::terminal_badge::{create_terminal_priority_badge, create_terminal_task_badges};
use crate::todoist::TaskDisplay;

/// Tasks list component
pub struct TasksList;

impl TasksList {
    /// Render the tasks list
    pub fn render(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
        if app.tasks.is_empty() {
            // Show empty state message
            let empty_message = if app.projects.is_empty() {
                "No projects available. Press 'r' to sync or 'A' to create a project."
            } else {
                "No tasks in this project. Press 'a' to create a task."
            };

            let empty_list = List::new(vec![ListItem::new(empty_message)]).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("{} Tasks", app.icons.tasks_title()))
                    .title_alignment(Alignment::Center),
            );

            f.render_stateful_widget(empty_list, area, &mut app.task_list_state.clone());
        } else {
            // Create list items with sections and tasks
            let items = Self::create_task_list_items(app, area);

            let tasks_list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!("{} Tasks", app.icons.tasks_title()))
                        .title_alignment(Alignment::Center),
                )
                .highlight_style(
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::White)
                        .add_modifier(Modifier::BOLD),
                );

            f.render_stateful_widget(tasks_list, area, &mut app.task_list_state.clone());
        }
    }

    /// Create list items with sections and tasks organized properly
    fn create_task_list_items(app: &App, area: ratatui::layout::Rect) -> Vec<ListItem<'_>> {
        let mut items = Vec::new();
        let mut task_index = 0;

        // Get the current project to filter sections
        let current_project = match &app.sidebar_selection {
            super::super::app::SidebarSelection::Project(index) => app.projects.get(*index).map(|p| &p.id),
            _ => None,
        };

        if let Some(project_id) = current_project {
            // Get sections for the current project
            let project_sections: Vec<_> = app
                .sections
                .iter()
                .filter(|section| section.project_id == *project_id)
                .collect();

            // Group tasks by section
            let mut tasks_by_section: std::collections::HashMap<Option<String>, Vec<&TaskDisplay>> =
                std::collections::HashMap::new();

            for task in &app.tasks {
                tasks_by_section
                    .entry(task.section_id.clone())
                    .or_default()
                    .push(task);
            }

            // Add tasks without sections first
            if let Some(tasks_without_section) = tasks_by_section.get(&None) {
                for task in tasks_without_section {
                    let item = Self::create_task_item(task, task_index, app);
                    items.push(item);
                    task_index += 1;
                }
            }

            // Add sections and their tasks
            for (section_index, section) in project_sections.iter().enumerate() {
                // Add section header (3 lines: blank, name, separator)
                // Only add blank line if there are already items in the list or this isn't the first section
                if !items.is_empty() || section_index > 0 {
                    items.push(ListItem::new(Line::from(""))); // Blank line
                }
                items.push(ListItem::new(Line::from(Span::styled(
                    section.name.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                )))); // Section name
                items.push(ListItem::new(Line::from(Span::styled(
                    "─".repeat(area.width as usize),
                    Style::default().fg(Color::Gray),
                )))); // Separator

                // Add tasks for this section
                if let Some(section_tasks) = tasks_by_section.get(&Some(section.id.clone())) {
                    for task in section_tasks {
                        let item = Self::create_task_item(task, task_index, app);
                        items.push(item);
                        task_index += 1;
                    }
                }
            }
        } else {
            // No project selected, show all tasks without sections
            for task in &app.tasks {
                let item = Self::create_task_item(task, task_index, app);
                items.push(item);
                task_index += 1;
            }
        }

        items
    }

    /// Create a single task item
    fn create_task_item<'a>(task: &'a TaskDisplay, index: usize, app: &'a App) -> ListItem<'a> {
        let is_selected = index == app.selected_task_index;

        // Create status indicator
        let status_icon = if task.is_deleted {
            app.icons.task_deleted()
        } else if task.is_completed {
            app.icons.task_completed()
        } else {
            app.icons.task_pending()
        };

        // Create priority badge using the proper function
        let priority_badge = create_terminal_priority_badge(task.priority);

        // Create badges for task metadata
        let metadata_badges = create_terminal_task_badges(
            task.is_recurring,
            task.due.is_some() || task.deadline.is_some(),
            task.duration.as_deref(),
            task.labels.as_slice(),
        );

        // Build the line with multiple spans for proper color rendering
        let mut line_spans = Vec::new();

        // Status icon
        let status_style = if task.is_deleted {
            Style::default().fg(Color::Red)
        } else if task.is_completed {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::White)
        };
        line_spans.push(Span::styled(format!("{} ", status_icon), status_style));

        // Priority badge (if any)
        if let Some(badge) = priority_badge {
            line_spans.push(badge.clone());
            line_spans.push(Span::raw(" "));
        }

        // Task content with appropriate styling
        let content_style = if task.is_deleted {
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::CROSSED_OUT)
        } else if task.is_completed {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::DIM)
        } else {
            Style::default().fg(Color::White)
        };
        line_spans.push(Span::styled(task.content.clone(), content_style));

        // Metadata badges
        for badge in metadata_badges {
            line_spans.push(Span::raw(" "));
            line_spans.push(badge.clone());
        }

        // Create the ListItem with proper styling
        let item_style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        ListItem::new(Line::from(line_spans)).style(item_style)
    }
}
