//! Repository layer for database operations.
//!
//! This module provides repository structs that encapsulate database queries
//! and operations, following the Data Mapper pattern recommended by SeaORM.
//! Repositories keep entities as pure data models while providing reusable
//! database access methods.

pub(crate) mod backend;
pub(crate) mod label;
pub(crate) mod project;
pub(crate) mod section;
pub(crate) mod task;

pub use backend::BackendRepository;
pub use label::LabelRepository;
pub use project::ProjectRepository;
pub use section::SectionRepository;
pub use task::TaskRepository;
