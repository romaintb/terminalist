//! Dialog components module

mod delete_confirmation_dialog;
mod error_dialog;
mod info_dialog;
mod project_creation_dialog;
mod project_delete_confirmation_dialog;
mod syncing_dialog;
mod task_creation_dialog;
mod task_edit_dialog;

pub use delete_confirmation_dialog::DeleteConfirmationDialog;
pub use error_dialog::ErrorDialog;
pub use info_dialog::InfoDialog;
pub use project_creation_dialog::ProjectCreationDialog;
pub use project_delete_confirmation_dialog::ProjectDeleteConfirmationDialog;
pub use syncing_dialog::SyncingDialog;
pub use task_creation_dialog::TaskCreationDialog;
pub use task_edit_dialog::TaskEditDialog;
