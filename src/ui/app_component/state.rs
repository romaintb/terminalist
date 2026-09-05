//! The data the app renders, kept apart from the components that draw it.

use crate::entities::{label, project, section, task};
use crate::ui::core::SidebarSelection;

/// Application state separate from UI concerns
#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub projects: Vec<project::Model>,
    pub tasks: Vec<task::Model>,
    pub labels: Vec<label::Model>,
    pub sections: Vec<section::Model>,
    pub sidebar_selection: SidebarSelection,
    pub loading: bool,
    pub show_help: bool,
    /// didnt we just got rid of custom scrolling ?
    pub help_scroll_offset: usize,
}

impl AppState {
    /// Update all data at once
    pub fn update_data(
        &mut self,
        projects: Vec<project::Model>,
        labels: Vec<label::Model>,
        sections: Vec<section::Model>,
        tasks: Vec<task::Model>,
    ) {
        self.projects = projects;
        self.labels = labels;
        self.sections = sections;
        self.tasks = tasks;
    }

    /// Whether the sidebar selection still names something that exists.
    ///
    /// A project or label deleted from another client is simply absent from the next sync,
    /// which leaves the selection pointing at nothing.
    pub fn selection_is_live(&self) -> bool {
        match self.sidebar_selection {
            SidebarSelection::Project(uuid) => self.projects.iter().any(|project| project.uuid == uuid),
            SidebarSelection::Label(uuid) => self.labels.iter().any(|label| label.uuid == uuid),
            SidebarSelection::Today | SidebarSelection::Tomorrow | SidebarSelection::Upcoming => true,
        }
    }
}
