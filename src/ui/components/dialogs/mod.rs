//! Dialog components module

mod error_dialog;
mod delete_confirmation_dialog;
mod project_creation_dialog;
mod task_creation_dialog;
mod project_delete_confirmation_dialog;

pub use error_dialog::ErrorDialog;
pub use delete_confirmation_dialog::DeleteConfirmationDialog;
pub use project_creation_dialog::ProjectCreationDialog;
pub use task_creation_dialog::TaskCreationDialog;
pub use project_delete_confirmation_dialog::ProjectDeleteConfirmationDialog;
