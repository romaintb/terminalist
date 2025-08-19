//! Label delete confirmation dialog component

use ratatui::{
    layout::Alignment,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::super::super::app::App;
use super::super::super::layout::LayoutManager;

/// Label delete confirmation dialog component
pub struct LabelDeleteConfirmationDialog;

impl LabelDeleteConfirmationDialog {
    /// Render the label delete confirmation dialog
    pub fn render(f: &mut Frame, app: &App) {
        if app.delete_label_confirmation.is_some() {
            let dialog_area = LayoutManager::centered_rect(50, 15, f.area());
            f.render_widget(Clear, dialog_area);

            // Find the label name for confirmation
            let label_name = if let Some(label_id) = &app.delete_label_confirmation {
                app.labels
                    .iter()
                    .find(|l| l.id == *label_id)
                    .map(|l| l.name.as_str())
                    .unwrap_or("Unknown")
            } else {
                "Unknown"
            };

            let confirmation_text =
                format!("Delete label '{label_name}'?\n\nPress 'y' to confirm, any other key to cancel");
            let paragraph = Paragraph::new(confirmation_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Confirm Label Deletion")
                        .title_alignment(Alignment::Center),
                )
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center);

            f.render_widget(paragraph, dialog_area);
        }
    }
}
