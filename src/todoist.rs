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
    pub id: String,
    /// Human-readable name of the project
    pub name: String,
    /// Color identifier for the project (used for UI theming)
    pub color: String,
    /// Whether this project is marked as a favorite
    pub is_favorite: bool,
    /// Optional parent project ID for sub-projects
    pub parent_id: Option<String>,
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
    pub id: String,
    /// Human-readable name of the label
    pub name: String,
    /// Color identifier for the label (used for UI theming)
    pub color: String,
}

/// Display model for sections optimized for UI rendering.
///
/// Sections are used to organize tasks within projects. This model contains
/// the section information needed for proper task organization and display.
#[derive(Debug, Clone)]
pub struct SectionDisplay {
    /// Unique identifier for the section
    pub id: String,
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
    pub id: String,
    /// Main content/description of the task
    pub content: String,
    /// ID of the project this task belongs to
    pub project_id: String,
    /// Optional section ID within the project
    pub section_id: Option<String>,
    /// Optional parent task ID for sub-tasks
    pub parent_id: Option<String>,
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

// Conversion implementations from API models to display models

/// Converts a Todoist API `Project` into a `ProjectDisplay` for UI use.
///
/// This implementation handles the transformation from the API's project model
/// to the flattened display model used throughout the application.
impl From<Project> for ProjectDisplay {
    fn from(project: Project) -> Self {
        Self {
            id: project.id,
            name: project.name,
            color: project.color,
            is_favorite: project.is_favorite,
            parent_id: project.parent_id,
            is_inbox_project: project.is_inbox_project,
        }
    }
}

/// Converts a Todoist API `Section` into a `SectionDisplay` for UI use.
///
/// This implementation handles the transformation from the API's section model
/// to the flattened display model used throughout the application.
impl From<Section> for SectionDisplay {
    fn from(section: Section) -> Self {
        Self {
            id: section.id,
            name: section.name,
            project_id: section.project_id,
            order: section.order,
        }
    }
}

/// Converts a Todoist API `Task` into a `TaskDisplay` for UI use.
///
/// This implementation handles the complex transformation from the API's task model
/// to the flattened display model. It includes special handling for:
/// - Duration formatting (converts API duration objects to human-readable strings)
/// - Label conversion (transforms label names to LabelDisplay objects)
/// - Date field extraction and formatting
/// - Recurring task detection
impl From<Task> for TaskDisplay {
    fn from(task: Task) -> Self {
        let duration_string = task.duration.map(|d| match d.unit.as_str() {
            "minute" => format!("{}m", d.amount),
            "hour" => format!("{}h", d.amount),
            "day" => format!("{}d", d.amount),
            _ => format!("{} {}", d.amount, d.unit),
        });

        // Convert label names to LabelDisplay objects (colors will be filled in later)
        let labels: Vec<LabelDisplay> = task
            .labels
            .into_iter()
            .map(|name| LabelDisplay {
                id: name.clone(), // Use name as ID for now
                name,
                color: "blue".to_string(), // Default color, will be updated from storage
            })
            .collect();

        Self {
            id: task.id,
            content: task.content,
            project_id: task.project_id,
            section_id: task.section_id,
            parent_id: task.parent_id,
            priority: task.priority,
            due: task.due.as_ref().map(|d| d.date.clone()),
            due_datetime: task.due.as_ref().and_then(|d| d.datetime.clone()),
            is_recurring: task.due.as_ref().is_some_and(|d| d.is_recurring),
            deadline: task.deadline.as_ref().map(|d| d.date.clone()),
            duration: duration_string,
            labels,
            description: task.description,
            is_completed: task.is_completed,
            is_deleted: false, // Tasks from API are never locally deleted
        }
    }
}
