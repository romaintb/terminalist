//! Task creation dialog component

use ratatui::{
    layout::Alignment,
    prelude::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::super::super::app::App;
use super::super::super::layout::LayoutManager;

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
                scaled_content_height,
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
                scaled_project_height,
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
                    scaled_instructions_height,
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
