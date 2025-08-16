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

            // Ensure we don't exceed dialog bounds
            let name_height = 6;
            let parent_height = 6;
            let instructions_height = 3;
            let total_height = name_height + parent_height + instructions_height + 4; // +4 for spacing
            
            // If dialog is too small, reduce heights
            let available_height = dialog_area.height.saturating_sub(4); // Account for borders
            let scale_factor = if available_height < total_height {
                available_height as f32 / total_height as f32
            } else {
                1.0
            };
            
            let scaled_name_height = (name_height as f32 * scale_factor).max(3.0) as u16;
            let scaled_parent_height = (parent_height as f32 * scale_factor).max(3.0) as u16;
            let scaled_instructions_height = (instructions_height as f32 * scale_factor).max(2.0) as u16;

            // Project name input - positioned at top with dynamic height
            let name_rect = Rect::new(
                dialog_area.x + 2,
                dialog_area.y + 2,
                dialog_area.width.saturating_sub(4),
                scaled_name_height
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

            // Parent project selection - positioned below name input with dynamic spacing
            let parent_y = dialog_area.y + 2 + scaled_name_height + 1;
            let parent_rect = Rect::new(
                dialog_area.x + 2,
                parent_y,
                dialog_area.width.saturating_sub(4),
                scaled_parent_height
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

            // Instructions - positioned at the bottom with bounds checking
            let instructions_y = parent_y + scaled_parent_height + 1;
            if instructions_y + scaled_instructions_height <= dialog_area.y + dialog_area.height {
                let instructions_rect = Rect::new(
                    dialog_area.x + 2,
                    instructions_y,
                    dialog_area.width.saturating_sub(4),
                    scaled_instructions_height
                );
                
                let instructions = "Press Enter to create, Esc to cancel, 'p' to select parent";
                let instructions_paragraph = Paragraph::new(instructions)
                    .style(Style::default().fg(Color::Yellow))
                    .alignment(Alignment::Center);
                f.render_widget(instructions_paragraph, instructions_rect);
            }
        }
    }
}

/// Task creation dialog component
pub struct TaskCreationDialog;

impl TaskCreationDialog {
    /// Render the task creation dialog
    pub fn render(f: &mut Frame, app: &App) {
        if app.creating_task {
            let dialog_area = LayoutManager::centered_rect(60, 20, f.size());
            f.render_widget(Clear, dialog_area);

            // Ensure we don't exceed dialog bounds
            let content_height = 6;
            let project_height = 6;
            let instructions_height = 3;
            let total_height = content_height + project_height + instructions_height + 4; // +4 for spacing
            
            // If dialog is too small, reduce heights
            let available_height = dialog_area.height.saturating_sub(4); // Account for borders
            let scale_factor = if available_height < total_height {
                available_height as f32 / total_height as f32
            } else {
                1.0
            };
            
            let scaled_content_height = (content_height as f32 * scale_factor).max(3.0) as u16;
            let scaled_project_height = (project_height as f32 * scale_factor).max(3.0) as u16;
            let scaled_instructions_height = (instructions_height as f32 * scale_factor).max(2.0) as u16;

            // Task content input - positioned at top with dynamic height
            let content_rect = Rect::new(
                dialog_area.x + 2,
                dialog_area.y + 2,
                dialog_area.width.saturating_sub(4),
                scaled_content_height
            );
            
            let content_text = if app.new_task_content.is_empty() {
                "Enter task content: "
            } else {
                &app.new_task_content
            };
            let content_paragraph = Paragraph::new(content_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("📝 New Task")
                        .title_alignment(Alignment::Center),
                )
                .style(Style::default().fg(Color::Green))
                .alignment(Alignment::Left);
            f.render_widget(content_paragraph, content_rect);

            // Project selection - positioned below content input with dynamic spacing
            let project_y = dialog_area.y + 2 + scaled_content_height + 1;
            let project_rect = Rect::new(
                dialog_area.x + 2,
                project_y,
                dialog_area.width.saturating_sub(4),
                scaled_project_height
            );
            
            let project_text = if let Some(project_id) = &app.new_task_project_id {
                if let Some(project) = app.projects.iter().find(|p| p.id == *project_id) {
                    format!("Project: {}", project.name)
                } else {
                    "Project: None (invalid)".to_string()
                }
            } else {
                "Project: None".to_string()
            };
            
            let project_paragraph = Paragraph::new(project_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Project")
                        .title_alignment(Alignment::Center),
                )
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Left);
            f.render_widget(project_paragraph, project_rect);

            // Instructions - positioned at the bottom with bounds checking
            let instructions_y = project_y + scaled_project_height + 1;
            if instructions_y + scaled_instructions_height <= dialog_area.y + dialog_area.height {
                let instructions_rect = Rect::new(
                    dialog_area.x + 2,
                    instructions_y,
                    dialog_area.width.saturating_sub(4),
                    scaled_instructions_height
                );
                
                let instructions = "Press Enter to create, Esc to cancel";
                let instructions_paragraph = Paragraph::new(instructions)
                    .style(Style::default().fg(Color::Yellow))
                    .alignment(Alignment::Center);
                f.render_widget(instructions_paragraph, instructions_rect);
            }
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
