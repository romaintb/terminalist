//! Repository layer for database operations.
//!
//! This module provides repository structs that encapsulate database queries
//! and operations, following the Data Mapper pattern recommended by SeaORM.
//! Repositories keep entities as pure data models while providing reusable
//! database access methods.

pub mod backend;
pub mod label;
pub mod project;
pub mod section;
pub mod task;

pub use backend::BackendRepository;
pub use label::LabelRepository;
pub use project::ProjectRepository;
pub use section::SectionRepository;
pub use task::TaskRepository;
