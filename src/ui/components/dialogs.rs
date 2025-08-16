//! Dialog components

use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::super::app::App;
use super::super::layout::LayoutManager;

/// Error dialog component
pub struct ErrorDialog;

impl ErrorDialog {
    /// Render the error dialog
    pub fn render(f: &mut Frame, app: &App) {
        if let Some(error_msg) = &app.error_message {
            let error_area = LayoutManager::centered_rect(60, 20, f.size());
            f.render_widget(Clear, error_area);
            let error_paragraph = Paragraph::new(error_msg.as_str())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Error")
                        .title_alignment(Alignment::Center),
                )
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            f.render_widget(error_paragraph, error_area);
        }
    }
}

/// Delete confirmation dialog component
pub struct DeleteConfirmationDialog;

impl DeleteConfirmationDialog {
    /// Render the delete confirmation dialog
    pub fn render(f: &mut Frame, app: &App) {
        if let Some(_task_id) = &app.delete_confirmation {
            if let Some(task) = app.tasks.get(app.selected_task_index) {
                let confirm_area = LayoutManager::centered_rect(60, 25, f.size());
                f.render_widget(Clear, confirm_area);

                let task_preview = if task.content.len() > 40 {
                    format!("{}...", &task.content[..37])
                } else {
                    task.content.clone()
                };

                let confirm_text = format!(
                    "Delete task?\n\n\"{task_preview}\"\n\nThis action cannot be undone!\n\nPress 'y' to confirm or 'n'/Esc to cancel",
                );

                let confirm_paragraph = Paragraph::new(confirm_text)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("⚠️  Confirm Delete")
                            .title_alignment(Alignment::Center),
                    )
                    .style(Style::default().fg(Color::Red))
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true });
                f.render_widget(confirm_paragraph, confirm_area);
            }
        }
    }
}

/// Project creation dialog component
pub struct ProjectCreationDialog;

impl ProjectCreationDialog {
    /// Render the project creation dialog
    pub fn render(f: &mut Frame, app: &App) {
        if app.creating_project {
            let dialog_area = LayoutManager::centered_rect(60, 20, f.size());
            f.render_widget(Clear, dialog_area);

            // Project name input - simple positioning
            let name_rect = Rect::new(
                dialog_area.x + 2,
                dialog_area.y + 2,
                dialog_area.width.saturating_sub(4),
                6
            );
            
            let name_text = if app.new_project_name.is_empty() {
                "Enter project name: "
            } else {
                &app.new_project_name
            };
            let name_paragraph = Paragraph::new(name_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("📁 New Project")
                        .title_alignment(Alignment::Center),
                )
                .style(Style::default().fg(Color::Green))
                .alignment(Alignment::Left);
            f.render_widget(name_paragraph, name_rect);

            // Parent project selection - positioned below name input
            let parent_rect = Rect::new(
                dialog_area.x + 2,
                dialog_area.y + 9,
                dialog_area.width.saturating_sub(4),
                6
            );
            
            let parent_text = if let Some(parent_id) = &app.new_project_parent_id {
                if let Some(parent) = app.projects.iter().find(|p| p.id == *parent_id) {
                    format!("Parent: {}", parent.name)
                } else {
                    "Parent: None (invalid)".to_string()
                }
            } else {
                "Parent: None\nPress 'p' to select".to_string()
            };
            
            let parent_paragraph = Paragraph::new(parent_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Parent Project")
                        .title_alignment(Alignment::Center),
                )
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Left);
            f.render_widget(parent_paragraph, parent_rect);

            // Instructions - positioned at the bottom
            let instructions_rect = Rect::new(
                dialog_area.x + 2,
                dialog_area.y + 16,
                dialog_area.width.saturating_sub(4),
                3
            );
            
            let instructions = "Press Enter to create, Esc to cancel, 'p' to select parent";
            let instructions_paragraph = Paragraph::new(instructions)
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center);
            f.render_widget(instructions_paragraph, instructions_rect);
        }
    }
}

/// Project deletion confirmation dialog component
pub struct ProjectDeleteConfirmationDialog;

impl ProjectDeleteConfirmationDialog {
    /// Render the project deletion confirmation dialog
    pub fn render(f: &mut Frame, app: &App) {
        if let Some(project_id) = &app.delete_project_confirmation {
            if let Some(project) = app.projects.iter().find(|p| p.id == *project_id) {
                let confirm_area = LayoutManager::centered_rect(70, 20, f.size());
                f.render_widget(Clear, confirm_area);

                let confirm_text = format!(
                    "Delete project?\n\n\"{}\"\n\nThis will also delete all tasks in this project!\n\nThis action cannot be undone!\n\nPress 'y' to confirm or 'n'/Esc to cancel",
                    project.name
                );

                let confirm_paragraph = Paragraph::new(confirm_text)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("⚠️  Confirm Project Delete")
                            .title_alignment(Alignment::Center),
                    )
                    .style(Style::default().fg(Color::Red))
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true });
                f.render_widget(confirm_paragraph, confirm_area);
            }
        }
    }
}
