//! Reusable UI components for the Terminalist application.
//!
//! This module contains a collection of composable UI components that can be
//! combined to create the complete user interface. Each component implements
//! the [`Component`] trait and handles its own rendering, state management,
//! and user interactions.
//!
//! # Component Categories
//!
//! ## Visual Elements
//! - [`badge`] - Small status indicators and labels
//!
//! ## Interactive Components
//! - [`dialog_component`] - Modal dialog framework
//! - [`dialogs`] - Specific dialog implementations (create, edit, confirm)
//! - [`sidebar_component`] - Navigation sidebar with project/view selection
//! - [`task_list_component`] - Main task display and management interface
//! - [`task_list_item_component`] - Individual task rendering and interaction
//!
//! # Design Principles
//!
//! All components follow these design principles:
//!
//! - **Composability** - Components can be nested and combined
//! - **Reusability** - Common patterns are extracted into reusable components
//! - **State isolation** - Each component manages its own internal state
//! - **Event delegation** - User interactions are passed up through callbacks
//! - **Consistent styling** - All components use the same color scheme and layout patterns
//!
//! # Usage
//!
//! Components are typically instantiated within the main application component
//! and rendered as part of the overall UI layout. They communicate with the
//! application through the shared [`AppContext`] and action system.

// Visual element components
pub mod badge;

// Utility components
pub mod scrollbar_helper;

// Core interactive components
pub mod dialog_component;
pub mod dialogs;
pub mod sidebar_component;
pub mod task_list_component;
pub mod task_list_item_component;

// Public exports for external use
pub use dialog_component::DialogComponent;
pub use sidebar_component::SidebarComponent;
pub use task_list_component::TaskListComponent;
