//! Terminalist - A Terminal User Interface (TUI) for Todoist
//!
//! This library provides a complete terminal-based interface for managing
//! Todoist tasks, projects, and labels. It includes synchronization with
//! the Todoist API, local storage, and a rich interactive UI built with
//! Ratatui.
//!
//! # Modules
//!
//! The library is organized into several key modules:
//!
//! * [`config`] - Application configuration management
//! * [`storage`] - Local database and data persistence
//! * [`sync`] - Synchronization with Todoist API
//! * [`todoist`] - Todoist API client and data structures
//! * [`ui`] - Terminal user interface components
//! * [`utils`] - Utility functions and helpers

/// Backend abstraction layer for multi-backend support
pub mod backend;

/// Configuration module for managing application settings
pub mod config;

/// Application constants and default values
pub mod constants;

/// SeaORM entity models for database tables
pub mod entities;

/// Icon definitions for visual representation in the TUI
pub mod icons;

/// Logging utilities for debugging and error tracking
pub mod logger;

/// Repository layer for database operations
pub mod repositories;

/// Local storage layer for caching Todoist data
pub mod storage;

/// Synchronization engine for keeping local and remote data in sync
pub mod sync;

/// Todoist API client and data models
pub mod todoist;

/// Terminal user interface components and rendering
pub mod ui;

/// Utility functions for date/time handling and other helpers
pub mod utils;

// Re-export entity models for convenient access
pub use entities::{label, project, section, task, task_label};
