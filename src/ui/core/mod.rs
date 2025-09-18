//! Core UI functionality for the Terminalist application.
//!
//! This module contains the fundamental building blocks for the user interface,
//! including event handling, state management, component abstractions, and task
//! management. It provides the foundation that all UI components build upon.
//!
//! # Module Components
//!
//! - [`actions`] - Action definitions and UI state transitions
//! - [`component`] - Base component trait and rendering abstractions
//! - [`context`] - Application context and shared state management
//! - [`event_handler`] - Event processing and keyboard/mouse input handling
//! - [`task_manager`] - Background task management and async operation handling
//!
//! # Architecture
//!
//! The core UI follows a component-based architecture where:
//!
//! 1. **Components** implement the [`Component`] trait for consistent rendering
//! 2. **Actions** define state transitions and user interactions
//! 3. **Context** provides shared application state and services
//! 4. **Events** are processed through the [`EventHandler`] system
//! 5. **Tasks** are managed asynchronously via the [`TaskManager`]
//!
//! This architecture ensures clean separation of concerns and makes the codebase
//! maintainable and testable.

// Core UI modules
pub mod actions;
pub mod component;
pub mod context;
pub mod event_handler;
pub mod task_manager;

// Re-export core types for easier access from other modules
pub use actions::{Action, DialogType, SidebarSelection};
pub use component::Component;
pub use context::AppContext;
pub use event_handler::{EventHandler, EventType};
pub use task_manager::{TaskId, TaskManager, TaskResult};
