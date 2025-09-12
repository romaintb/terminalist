use crate::sync::SyncStatus;

/// Represents the currently selected item in the sidebar
#[derive(Debug, Clone, PartialEq, Default)]
pub enum SidebarSelection {
    #[default]
    Today, // Today view (special view)
    Tomorrow,       // Tomorrow view (special view)
    Upcoming,       // Upcoming view (tasks with future due dates)
    Label(usize),   // Index into labels vector
    Project(usize), // Index into projects vector
}

#[derive(Debug, Clone)]
pub enum Action {
    // Navigation
    NavigateToSidebar(SidebarSelection),
    NextTask,
    PreviousTask,

    // Task operations
    ToggleTask(String),
    DeleteTask(String),
    CyclePriority(String),
    SetTaskDueToday(String),
    SetTaskDueTomorrow(String),
    SetTaskDueNextWeek(String),
    SetTaskDueWeekEnd(String),
    CreateTask {
        content: String,
        project_id: Option<String>,
    },
    EditTask {
        id: String,
        content: String,
    },

    // Project operations
    CreateProject {
        name: String,
        parent_id: Option<String>,
    },
    EditProject {
        id: String,
        name: String,
    },
    DeleteProject(String),

    // Label operations
    CreateLabel {
        name: String,
    },
    EditLabel {
        id: String,
        name: String,
    },
    DeleteLabel(String),

    // Sync operations
    StartSync,
    SyncCompleted(SyncStatus),
    SyncFailed(String),
    DataLoaded {
        projects: Vec<crate::todoist::ProjectDisplay>,
        labels: Vec<crate::todoist::LabelDisplay>,
        sections: Vec<crate::todoist::SectionDisplay>,
        tasks: Vec<crate::todoist::TaskDisplay>,
    },

    // Data refresh after task operations
    RefreshData,

    // UI operations
    ShowHelp(bool),
    ShowDebug(bool),
    ShowDialog(DialogType),
    HideDialog,
    HelpScrollUp,
    HelpScrollDown,
    HelpScrollToTop,
    HelpScrollToBottom,

    // App control
    Quit,
    None,
}

#[derive(Debug, Clone)]
pub enum DialogType {
    TaskCreation {
        default_project_id: Option<String>,
    },
    TaskEdit {
        task_id: String,
        content: String,
        project_id: String,
    },
    ProjectCreation,
    ProjectEdit {
        project_id: String,
        name: String,
    },
    LabelCreation,
    LabelEdit {
        label_id: String,
        name: String,
    },
    DeleteConfirmation {
        item_type: String,
        item_id: String,
    },
    Error(String),
    Info(String),
    Help,
    Logs,
}
