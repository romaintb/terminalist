//! Tasks list component

use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use super::super::app::App;
use crate::terminal_badge::{create_terminal_task_badges, create_terminal_priority_badge};

/// Tasks list component
pub struct TasksList;

impl TasksList {
    /// Render the tasks list
    pub fn render(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
        if app.loading {
            let loading_text = Paragraph::new("Loading...")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("📝 Tasks")
                        .title_alignment(Alignment::Center),
                )
                .alignment(Alignment::Center);
            f.render_widget(loading_text, area);
        } else if app.tasks.is_empty() {
            let empty_text = Paragraph::new("No tasks in this project")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("📝 Tasks")
                        .title_alignment(Alignment::Center),
                )
                .alignment(Alignment::Center);
            f.render_widget(empty_text, area);
        } else {
            // Create list items from tasks using Line::from with multiple Spans
            let items: Vec<ListItem> = app
                .tasks
                .iter()
                .enumerate()
                .map(|(index, task)| {
                    let is_selected = index == app.selected_task_index;
                    
                    // Create status indicator
                    let status_icon = if task.is_deleted { "❌" } else if task.is_completed { "✅" } else { "🔳" };
                    
                    // Create priority badge using the proper function
                    let priority_badge = create_terminal_priority_badge(task.priority);
                    
                    // Create badges for task metadata
                    let metadata_badges = create_terminal_task_badges(
                        task.is_recurring,
                        task.due.is_some() || task.deadline.is_some(),
                        task.duration.as_deref(),
                        task.labels.len(),
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
                    line_spans.push(Span::styled(
                        format!("{} ", status_icon),
                        status_style
                    ));
                    
                    // Priority badge (if any)
                    if let Some(badge) = priority_badge {
                        line_spans.push(badge);
                        line_spans.push(Span::raw(" "));
                    }
                    
                    // Task content with appropriate styling
                    let content_style = if task.is_deleted {
                        Style::default().fg(Color::Red).add_modifier(Modifier::CROSSED_OUT)
                    } else if task.is_completed {
                        Style::default().fg(Color::Green).add_modifier(Modifier::DIM)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    line_spans.push(Span::styled(task.content.clone(), content_style));
                    
                    // Metadata badges
                    for badge in metadata_badges {
                        line_spans.push(Span::raw(" "));
                        line_spans.push(badge);
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
                })
                .collect();
            
            let tasks_list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("📝 Tasks")
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
}
