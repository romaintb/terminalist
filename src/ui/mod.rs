//! UI module for Terminalist
//!
//! This module handles all user interface components, rendering, and user interactions.

pub mod app;
pub mod components;
pub mod events;
pub mod layout;
pub mod renderer;

pub use app::App;
pub use events::handle_events;
pub use layout::LayoutManager;
pub use renderer::run_app;
