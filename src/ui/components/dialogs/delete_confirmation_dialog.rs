//! Delete confirmation dialog component

use ratatui::{
    layout::Alignment,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::super::super::app::App;
use super::super::super::layout::LayoutManager;

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
