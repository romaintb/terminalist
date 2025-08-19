//! Task edit dialog component

use ratatui::{
    layout::Alignment,
    prelude::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::super::super::app::App;
use super::super::super::layout::LayoutManager;

/// Task edit dialog component
pub struct TaskEditDialog;

impl TaskEditDialog {
    /// Render the task edit dialog
    pub fn render(f: &mut Frame, app: &App) {
        if app.editing_task {
            let dialog_area = LayoutManager::centered_rect(60, 20, f.area());
            f.render_widget(Clear, dialog_area);

            // Ensure we don't exceed dialog bounds
            let content_height = 6;
            let instructions_height = 3;
            let total_height = content_height + instructions_height + 2; // +2 for spacing

            // If dialog is too small, reduce heights
            let available_height = dialog_area.height.saturating_sub(4); // Account for borders
            let scale_factor = if available_height < total_height {
                f32::from(available_height) / f32::from(total_height)
            } else {
                1.0
            };

            let scaled_content_height = (f32::from(content_height) * scale_factor).max(3.0) as u16;
            let scaled_instructions_height = (f32::from(instructions_height) * scale_factor).max(2.0) as u16;

            // Task content input - positioned at top with dynamic height
            let content_rect = Rect::new(
                dialog_area.x + 2,
                dialog_area.y + 2,
                dialog_area.width.saturating_sub(4),
                scaled_content_height,
            );

            let content_text = if app.edit_task_content.is_empty() {
                "Enter task content: "
            } else {
                &app.edit_task_content
            };
            let content_paragraph = Paragraph::new(content_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("✏️ Edit Task")
                        .title_alignment(Alignment::Center),
                )
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Left);
            f.render_widget(content_paragraph, content_rect);

            // Instructions - positioned at the bottom with bounds checking
            let instructions_y = dialog_area.y + 2 + scaled_content_height + 1;
            if instructions_y + scaled_instructions_height <= dialog_area.y + dialog_area.height {
                let instructions_rect = Rect::new(
                    dialog_area.x + 2,
                    instructions_y,
                    dialog_area.width.saturating_sub(4),
                    scaled_instructions_height,
                );

                let instructions = "Press Enter to save, Esc to cancel";
                let instructions_paragraph = Paragraph::new(instructions)
                    .style(Style::default().fg(Color::Yellow))
                    .alignment(Alignment::Center);
                f.render_widget(instructions_paragraph, instructions_rect);
            }
        }
    }
}
