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
pub const SUCCESS_TASK_PRIORITY_UPDATED: &str = "✅ Task priority updated to P";
pub const SUCCESS_TASK_RESTORED: &str = "✅ Task restored";
pub const SUCCESS_SYNC_COMPLETED: &str = "✅ Synced";

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
pub const ERROR_TASK_RESTORE_FAILED: &str = "❌ Failed to restore task";

// Validation Error Messages

// Log Messages

// UI Messages
pub const CONFIG_GENERATED: &str = "✅ Generated default configuration file";
pub const UI_NO_TASK_SELECTED_DUE_DATE: &str = "No task selected to set due date";
pub const UI_LOADING_DATA: &str = "Loading data";
pub const UI_LOADING_DATA_FROM_STORAGE: &str = "Loading data from storage";

/// How long a success toast lingers before it fades on its own.
pub const TOAST_TTL_SECS: u64 = 3;
/// Failures get longer: they carry information the user has to actually read.
pub const TOAST_ERROR_TTL_SECS: u64 = 15;

// UI Layout Constants (width in columns)
pub const SIDEBAR_MIN_WIDTH: u16 = 15;
pub const SIDEBAR_MAX_WIDTH: u16 = 50;
pub const SIDEBAR_DEFAULT_WIDTH: u16 = 30;
pub const MAIN_AREA_MIN_WIDTH: u16 = 20;

pub const MEMORY_LOGS_LIMIT: usize = 5000;
