//! Todoist API integration for the terminalist application.
//!
//! This module provides a bridge between the external `todoist_api` crate and the
//! terminalist application. It re-exports the Todoist API functionality.

// Re-export the Todoist API library for external use
pub use todoist_api::*;
