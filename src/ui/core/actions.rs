use crate::sync::SyncStatus;
use crate::ui::app::SidebarSelection;

#[derive(Debug, Clone)]
pub enum Action {
    // Navigation
    NavigateToSidebar(SidebarSelection),
    NextTask,
    PreviousTask,

    // Task operations
    ToggleTask(String),
    DeleteTask(String),
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

    // App control
    Quit,
    None,
}

#[derive(Debug, Clone)]
pub enum DialogType {
    TaskCreation { default_project_id: Option<String> },
    TaskEdit { task_id: String, content: String },
    ProjectCreation,
    ProjectEdit { project_id: String, name: String },
    LabelCreation,
    LabelEdit { label_id: String, name: String },
    DeleteConfirmation { item_type: String, item_id: String },
    Error(String),
    Info(String),
    Help,
    Logs,
}
