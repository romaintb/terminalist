//! Utility modules for the Terminalist application.
//!
//! This module contains common utility functions and helpers that are used
//! throughout the application. These utilities provide functionality for
//! date/time handling and other cross-cutting concerns.
//!
//! # Available Utilities
//!
//! - [`datetime`] - Date and time formatting, parsing, and manipulation functions
//!
//! # Purpose
//!
//! The utilities in this module are designed to:
//!
//! - **Centralize common functionality** - Avoid code duplication across modules
//! - **Provide consistent interfaces** - Standardize how dates, etc. are handled
//! - **Abstract platform differences** - Handle cross-platform concerns in one place
//! - **Simplify complex operations** - Provide easy-to-use wrappers for complex tasks
//!
//! # Design Philosophy
//!
//! All utilities follow these principles:
//!
//! - **Pure functions** when possible - Avoid side effects for predictable behavior
//! - **Error handling** - Proper error types and handling for robust operation
//! - **Performance** - Efficient implementations suitable for frequent use
//! - **Testability** - Easy to unit test with clear inputs and outputs

pub mod datetime;
