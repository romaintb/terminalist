//! Local storage layer for Todoist data
//!
//! This module provides a SQLite-based local storage system for caching
//! Todoist data. It supports offline access and reduces API calls by
//! storing tasks, projects, labels, and sections locally.
//!
//! # Architecture
//!
//! The storage layer uses SQLite with separate tables for each data type:
//! * Tasks - Main task items with all properties
//! * Projects - Project hierarchy and metadata
//! * Labels - User-defined labels for categorization
//! * Sections - Project sections for task grouping

/// Database connection and core storage operations
pub mod db;

/// Backend registration and management storage
pub mod backends;

/// Label storage and color management
pub mod labels;

/// Project storage and hierarchy management
pub mod projects;

/// Section storage for project organization
pub mod sections;

/// Task storage and querying operations
pub mod tasks;

// Re-export the main types and struct
/// Main storage interface for all data operations
pub use db::LocalStorage;

/// Backend registration and status information
pub use backends::RegisteredBackend;

/// Local representation of a Todoist label with color information
pub use labels::{LocalLabel, LocalLabelColor};

/// Local representation of a Todoist project
pub use projects::LocalProject;

/// Local representation of a project section
pub use sections::LocalSection;

/// Local representation of a Todoist task with all properties
pub use tasks::LocalTask;
