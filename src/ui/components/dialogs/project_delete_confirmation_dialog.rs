//! Project deletion confirmation dialog component

use ratatui::{
    layout::Alignment,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::super::super::app::App;
use super::super::super::layout::LayoutManager;

/// Project deletion confirmation dialog component
pub struct ProjectDeleteConfirmationDialog;

impl ProjectDeleteConfirmationDialog {
    /// Render the project deletion confirmation dialog
    pub fn render(f: &mut Frame, app: &App) {
        if let Some(project_id) = &app.delete_project_confirmation {
            if let Some(project) = app.projects.iter().find(|p| p.id == *project_id) {
                let confirm_area = LayoutManager::centered_rect(70, 20, f.area());
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
