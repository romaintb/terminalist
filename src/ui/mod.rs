//! User Interface module for the Terminalist application.
//!
//! This module contains all user interface components, rendering logic, and user interaction
//! handling for the terminal-based Todoist client. It provides a comprehensive UI framework
//! built on top of the `ratatui` crate for creating rich terminal user interfaces.
//!
//! # Module Structure
//!
//! - [`app_component`] - Main application component that orchestrates the entire UI
//! - [`components`] - Reusable UI components like lists, forms, and dialogs
//! - [`core`] - Core UI functionality including state management and event handling
//! - [`layout`] - Layout management and responsive design utilities
//! - [`renderer`] - Terminal rendering engine and application runtime
//!
//! # Key Features
//!
//! - **Component-based architecture** - Modular, reusable UI components
//! - **State management** - Centralized application state with proper updates
//! - **Event handling** - Keyboard and mouse input processing
//! - **Responsive design** - Adaptive layouts for different terminal sizes
//! - **Theme support** - Consistent styling and color schemes
//!
//! # Usage
//!
//! The main entry point for the UI is through the [`run_app`] function, which initializes
//! the terminal interface and starts the application event loop:
//!
//! ```rust,no_run
//! use terminalist::ui::run_app;
//! use terminalist::sync::SyncService;
//! use terminalist::config::Config;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = Config::load()?;
//! let sync_service = SyncService::new("token".to_string(), false).await?;
//!
//! run_app(sync_service, config).await?;
//! # Ok(())
//! # }
//! ```

// Sub-modules containing UI functionality
pub mod app_component;
pub mod components;
pub mod core;
pub mod layout;
pub mod renderer;

// Re-export main UI types for external use
pub use app_component::AppComponent;
pub use layout::LayoutManager;
pub use renderer::run_app;
