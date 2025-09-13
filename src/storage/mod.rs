pub mod db;
pub mod labels;
pub mod projects;
pub mod sections;
pub mod tasks;

// Re-export the main types and struct
pub use db::LocalStorage;
pub use labels::{LocalLabel, LocalLabelColor};
pub use projects::LocalProject;
pub use sections::LocalSection;
pub use tasks::LocalTask;
