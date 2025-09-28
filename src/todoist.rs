//! Todoist API integration and display models for the terminalist application.
//!
//! This module provides a bridge between the external `todoist_api` crate and the
//! terminalist application. It re-exports the Todoist API functionality and defines
//! display models that are optimized for UI rendering and local storage.
//!
//! The module includes:
//! - Re-exports of the Todoist API client and types
//! - Display models for projects, tasks, labels, and sections
//! - Conversion implementations from API models to display models
//!
//! The display models are designed to be lightweight and contain only the data
//! needed for UI operations, with proper type conversions for dates, durations,
//! and other complex fields.

// Re-export the Todoist API library for external use
pub use todoist_api::*;

/// Display model for projects optimized for UI rendering.
///
/// This struct contains all the project information needed for UI display,
/// derived from the Todoist API's `Project` model but flattened for easier use.
/// It includes visual properties like color and favorite status for proper
/// UI presentation.
#[derive(Debug, Clone)]
pub struct ProjectDisplay {
    /// Unique identifier for the project
    pub uuid: String,
    /// Human-readable name of the project
    pub name: String,
    /// Color identifier for the project (used for UI theming)
    pub color: String,
    /// Whether this project is marked as a favorite
    pub is_favorite: bool,
    /// Optional parent project ID for sub-projects
    pub parent_uuid: Option<String>,
    /// Whether this is the special "Inbox" project
    pub is_inbox_project: bool,
}

/// Display model for labels optimized for UI rendering.
///
/// Labels are used to categorize and filter tasks. This model contains
/// the essential label information needed for UI display and user interaction.
#[derive(Debug, Clone)]
pub struct LabelDisplay {
    /// Unique identifier for the label
    pub uuid: String,
    /// Human-readable name of the label
    pub name: String,
    /// Color identifier for the label (used for UI theming)
    pub color: String,
}

/// Display model for sections optimized for UI rendering.
///
/// Sections are used to organize tasks within projects. This model contains
/// the section information needed for proper task organization and display.
#[derive(Debug, Clone)]  // UI still expects 'id' field
pub struct SectionDisplay {
    /// Unique identifier for the section
    pub uuid: String,
    /// Human-readable name of the section
    pub name: String,
    /// ID of the project this section belongs to
    pub project_id: String,
    /// Display order within the project
    pub order: i32,
}

/// Display model for tasks optimized for UI rendering.
///
/// This is the primary data structure for task display in the UI. It contains
/// all the task information needed for rendering, filtering, and user interaction.
/// The model flattens complex API structures and provides convenient access to
/// commonly used task properties.
///
/// # Fields
/// - Date fields are stored as strings for easy display formatting
/// - Labels are embedded as `LabelDisplay` objects for immediate access
/// - Duration is formatted as a human-readable string
/// - Priority follows Todoist's 1-4 scale (1=normal, 4=urgent)
#[derive(Debug, Clone)]
pub struct TaskDisplay {
    /// Unique identifier for the task
    pub uuid: String,
    /// Main content/description of the task
    pub content: String,
    /// ID of the project this task belongs to
    pub project_id: String,
    /// Optional section ID within the project
    pub section_id: Option<String>,
    /// Optional parent task ID for sub-tasks
    pub parent_uuid: Option<String>,
    /// Priority level (1=normal, 2=high, 3=very high, 4=urgent)
    pub priority: i32,
    /// Due date in YYYY-MM-DD format
    pub due: Option<String>,
    /// Due datetime in ISO 8601 format
    pub due_datetime: Option<String>,
    /// Whether this task has a recurring due date
    pub is_recurring: bool,
    /// Deadline date in YYYY-MM-DD format
    pub deadline: Option<String>,
    /// Human-readable duration string (e.g., "2h", "30m")
    pub duration: Option<String>,
    /// List of labels associated with this task
    pub labels: Vec<LabelDisplay>,
    /// Additional description or notes for the task
    pub description: String,
    /// Whether this task is completed
    pub is_completed: bool,
    /// Whether this task is soft-deleted locally
    pub is_deleted: bool,
}

// NOTE: Direct conversion from API objects to Display models has been removed.
// Display models should only be created from Local storage objects through the storage layer.
// This ensures proper UUID assignment and maintains the clean separation between:
// - Raw API objects (handled by backends and sync service)
// - Local storage objects (managed by storage layer with UUIDs and backend tracking)
// - Display models (used by UI, sourced only from storage layer)
