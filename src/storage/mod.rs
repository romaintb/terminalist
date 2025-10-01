//! Local storage module for managing Todoist data persistence
//!
//! This module provides database operations using SeaORM for:
//! - Projects
//! - Sections
//! - Tasks
//! - Labels
//! - Task-label relationships

pub mod db;
pub mod labels;
pub mod projects;
pub mod sections;
pub mod tasks;

pub use db::LocalStorage;
