pub mod actions;
pub mod component;
pub mod context;
pub mod event_handler;
pub mod task_manager;

// Re-exports for easier access
pub use actions::{Action, DialogType, SidebarSelection};
pub use component::Component;
pub use context::AppContext;
pub use event_handler::{EventHandler, EventType};
pub use task_manager::{TaskId, TaskManager, TaskResult};
