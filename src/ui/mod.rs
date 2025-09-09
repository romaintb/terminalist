//! UI module for Terminalist
//!
//! This module handles all user interface components, rendering, and user interactions.

pub mod app_component;
pub mod components;
pub mod core;
pub mod layout;
pub mod renderer;

pub use app_component::AppComponent;
pub use layout::LayoutManager;
pub use renderer::run_app;
