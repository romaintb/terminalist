//! UI module for Terminalist
//! 
//! This module handles all user interface components, rendering, and user interactions.

pub mod app;
pub mod renderer;
pub mod events;
pub mod layout;
pub mod components;

pub use app::App;
pub use renderer::run_app;
pub use events::handle_events;
pub use layout::LayoutManager;
