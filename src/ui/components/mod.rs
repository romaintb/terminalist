//! Reusable UI components

pub mod badge;
pub mod help_panel;
pub mod projects_list;
pub mod status_bar;
pub mod tasks_list;

// New component architecture
pub mod dialog_component;
pub mod sidebar_component;
pub mod task_list_component;

// Legacy dialog components (now handled by DialogComponent)
pub use help_panel::HelpPanel;
pub use projects_list::Sidebar;
pub use status_bar::StatusBar;
pub use tasks_list::TasksList;

// New component architecture exports
pub use dialog_component::DialogComponent;
pub use sidebar_component::SidebarComponent;
pub use task_list_component::TaskListComponent;
