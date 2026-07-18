use super::actions::{NavigationCounts, SidebarSelection};
use crate::entities::{label, project, section, task};

/// A complete, internally consistent result of one requested view load.
#[derive(Debug, Clone)]
pub struct ViewSnapshot {
    pub generation: u64,
    pub selection: SidebarSelection,
    pub is_initial: bool,
    pub projects: Vec<project::Model>,
    pub labels: Vec<label::Model>,
    pub sections: Vec<section::Model>,
    pub tasks: Vec<task::Model>,
    pub all_tasks: Vec<task::Model>,
    pub navigation_counts: NavigationCounts,
}
