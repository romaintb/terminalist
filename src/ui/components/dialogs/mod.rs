//! Dialog components module

mod delete_confirmation_dialog;
mod error_dialog;
mod info_dialog;
mod label_creation_dialog;
mod label_delete_confirmation_dialog;
mod label_edit_dialog;
mod project_creation_dialog;
mod project_delete_confirmation_dialog;
mod project_edit_dialog;
mod syncing_dialog;
mod task_creation_dialog;
mod task_edit_dialog;

pub use delete_confirmation_dialog::DeleteConfirmationDialog;
pub use error_dialog::ErrorDialog;
pub use info_dialog::InfoDialog;
pub use label_creation_dialog::LabelCreationDialog;
pub use label_delete_confirmation_dialog::LabelDeleteConfirmationDialog;
pub use label_edit_dialog::LabelEditDialog;
pub use project_creation_dialog::ProjectCreationDialog;
pub use project_delete_confirmation_dialog::ProjectDeleteConfirmationDialog;
pub use project_edit_dialog::ProjectEditDialog;
pub use syncing_dialog::SyncingDialog;
pub use task_creation_dialog::TaskCreationDialog;
pub use task_edit_dialog::TaskEditDialog;
