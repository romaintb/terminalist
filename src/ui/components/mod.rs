//! Reusable UI components

pub mod badge;

// Component architecture
pub mod dialog_component;
pub mod dialogs;
pub mod sidebar_component;
pub mod task_list_component;

// Component exports
pub use dialog_component::DialogComponent;
pub use sidebar_component::SidebarComponent;
pub use task_list_component::TaskListComponent;
