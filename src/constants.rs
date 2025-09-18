//! Constants used throughout the application
//!
//! This module centralizes magic strings, UI text, and other constant values
//! to improve maintainability and consistency.

// UI Section Headers
pub const HEADER_OVERDUE: &str = "⏰ Overdue";
pub const HEADER_TODAY: &str = "📅 Today";
pub const HEADER_TOMORROW: &str = "📅 Tomorrow";

// Success Messages
pub const SUCCESS_TASK_COMPLETED: &str = "✅ Task completed";
pub const SUCCESS_TASK_DELETED: &str = "✅ Task deleted";
pub const SUCCESS_TASK_UPDATED: &str = "✅ Task updated";
pub const SUCCESS_TASK_CREATED_PROJECT: &str = "✅ Task created in project";
pub const SUCCESS_TASK_CREATED_INBOX: &str = "✅ Task created in inbox";
pub const SUCCESS_TASK_DUE_TODAY: &str = "✅ Task due date set to today";
pub const SUCCESS_TASK_DUE_TOMORROW: &str = "✅ Task due date set to tomorrow";
pub const SUCCESS_TASK_DUE_MONDAY: &str = "✅ Task due date set to next Monday";
pub const SUCCESS_TASK_DUE_SATURDAY: &str = "✅ Task due date set to next Saturday";
pub const SUCCESS_PROJECT_CREATED_PARENT: &str = "✅ Project created with parent";
pub const SUCCESS_PROJECT_CREATED_ROOT: &str = "✅ Root project created";
pub const SUCCESS_PROJECT_DELETED: &str = "✅ Project deleted";
pub const SUCCESS_PROJECT_UPDATED: &str = "✅ Project updated";
pub const SUCCESS_LABEL_CREATED: &str = "✅ Label created";
pub const SUCCESS_LABEL_DELETED: &str = "✅ Label deleted";
pub const SUCCESS_LABEL_UPDATED: &str = "✅ Label updated";

// Error Messages
pub const ERROR_TASK_COMPLETION_FAILED: &str = "❌ Failed to complete task";
pub const ERROR_TASK_DELETE_FAILED: &str = "❌ Failed to delete task";
pub const ERROR_TASK_UPDATE_FAILED: &str = "❌ Failed to update task";
pub const ERROR_TASK_CREATE_FAILED: &str = "❌ Failed to create task";
pub const ERROR_TASK_DUE_DATE_FAILED: &str = "❌ Failed to set task due date";
pub const ERROR_TASK_PRIORITY_FAILED: &str = "❌ Failed to update task priority";
pub const ERROR_PROJECT_CREATE_FAILED: &str = "❌ Failed to create project";
pub const ERROR_PROJECT_DELETE_FAILED: &str = "❌ Failed to delete project";
pub const ERROR_PROJECT_UPDATE_FAILED: &str = "❌ Failed to update project";
pub const ERROR_LABEL_CREATE_FAILED: &str = "❌ Failed to create label";
pub const ERROR_LABEL_DELETE_FAILED: &str = "❌ Failed to delete label";
pub const ERROR_LABEL_UPDATE_FAILED: &str = "❌ Failed to update label";

// Validation Error Messages
pub const ERROR_INVALID_PRIORITY_FORMAT: &str = "❌ Invalid priority value format";
pub const ERROR_INVALID_PRIORITY_INFO: &str = "❌ Invalid task priority info format";
pub const ERROR_INVALID_DATE_FORMAT: &str = "❌ Invalid task info format for setting due date";
pub const ERROR_INVALID_TASK_EDIT_FORMAT: &str = "❌ Invalid task edit format";
pub const ERROR_INVALID_PROJECT_EDIT_FORMAT: &str = "❌ Invalid project edit format";
pub const ERROR_INVALID_LABEL_EDIT_FORMAT: &str = "❌ Invalid label edit format";
pub const ERROR_UNKNOWN_OPERATION: &str = "❌ Unknown operation";

// Log Messages
pub const LOG_FETCHED_PROJECTS: &str = "✅ Fetched {} projects from API";
pub const LOG_FETCHED_TASKS: &str = "✅ Fetched {} tasks from API";
pub const LOG_FETCHED_LABELS: &str = "✅ Fetched {} labels from API";
pub const LOG_FETCHED_SECTIONS: &str = "✅ Fetched {} sections from API";
pub const LOG_STORED_PROJECTS: &str = "✅ Stored projects in database";
pub const LOG_STORED_TASKS: &str = "✅ Stored tasks in database";
pub const LOG_STORED_LABELS: &str = "✅ Stored labels in database";
pub const LOG_STORED_SECTIONS: &str = "✅ Stored sections in database";
pub const LOG_ERROR_FETCH_PROJECTS: &str = "❌ Failed to fetch projects";
pub const LOG_ERROR_FETCH_TASKS: &str = "❌ Failed to fetch tasks";
pub const LOG_ERROR_FETCH_LABELS: &str = "❌ Failed to fetch labels";
pub const LOG_ERROR_FETCH_SECTIONS: &str = "❌ Failed to fetch sections";
pub const LOG_ERROR_STORE_PROJECTS: &str = "❌ Failed to store projects";
pub const LOG_ERROR_STORE_TASKS: &str = "❌ Failed to store tasks";
pub const LOG_ERROR_STORE_LABELS: &str = "❌ Failed to store labels";
pub const LOG_ERROR_STORE_SECTIONS: &str = "❌ Failed to store sections";

// UI Messages
pub const CONFIG_GENERATED: &str = "✅ Generated default configuration file";
pub const ERROR_NO_API_TOKEN: &str = "❌ Error: TODOIST_API_TOKEN environment variable not set";
pub const DIALOG_TITLE_DEBUG_LOGS: &str = "🔍 Debug Logs - Press 'Esc', 'G' or 'q' to close";

// Date header format for upcoming view
pub const UPCOMING_DATE_FORMAT: &str = "📊 {} - {}";

// UI Layout Constants
/// Minimum sidebar width in columns
pub const SIDEBAR_MIN_WIDTH: u16 = 15;
/// Maximum sidebar width in columns
pub const SIDEBAR_MAX_WIDTH: u16 = 50;
/// Default sidebar width in columns
pub const SIDEBAR_DEFAULT_WIDTH: u16 = 30;
/// Minimum main area width to preserve usability
pub const MAIN_AREA_MIN_WIDTH: u16 = 20;
