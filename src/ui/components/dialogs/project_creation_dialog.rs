//! Project creation dialog component

use ratatui::{
    layout::Alignment,
    prelude::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::super::super::app::App;
use super::super::super::layout::LayoutManager;

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
                scaled_name_height,
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
                scaled_parent_height,
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
                    scaled_instructions_height,
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
