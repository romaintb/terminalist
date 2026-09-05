use crate::sync::SyncStatus;
use uuid::Uuid;

/// Represents the currently selected item in the sidebar.
///
/// Projects and labels are named by their local UUID rather than by a position in the loaded
/// vectors. A sync that adds, removes or reorders a project would otherwise slide the
/// selection onto a different one without anything noticing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SidebarSelection {
    #[default]
    Today, // Today view (special view)
    Tomorrow, // Tomorrow view (special view)
    Upcoming, // Upcoming view (tasks with future due dates)
    Label(Uuid),
    Project(Uuid),
}

/// Why a data load was scheduled; decides what happens to the cursor when the reload lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadKind {
    /// First load of the session; resolves the configured `default_project`.
    Initial,
    /// User-triggered (navigation, task op, debug refresh). Cursor stays on its row.
    User,
    /// Background sync. Cursor re-anchors to the task it was on.
    Background,
}

#[derive(Debug, Clone)]
pub enum Action {
    // Navigation
    NavigateToSidebar(SidebarSelection),
    NextTask,
    PreviousTask,

    // Task operations
    CompleteTask(Uuid),
    DeleteTask(Uuid),
    CyclePriority(Uuid),
    SetTaskDueToday(Uuid),
    SetTaskDueTomorrow(Uuid),
    SetTaskDueNextWeek(Uuid),
    SetTaskDueWeekEnd(Uuid),
    CreateTask {
        content: String,
        project_uuid: Option<Uuid>,
    },
    EditTask {
        task_uuid: Uuid,
        content: String,
    },
    RestoreTask(Uuid),

    // Project operations
    CreateProject {
        name: String,
        parent_uuid: Option<Uuid>,
    },
    EditProject {
        project_uuid: Uuid,
        name: String,
    },
    DeleteProject(Uuid),

    // Label operations
    CreateLabel {
        name: String,
    },
    EditLabel {
        label_uuid: Uuid,
        name: String,
    },
    DeleteLabel(Uuid),

    // Sync operations
    StartSync,
    RefreshLocalData, // Debug mode: refresh from local DB without API sync
    SyncCompleted(SyncStatus),
    SyncFailed(String),
    DataLoaded {
        kind: LoadKind,
        projects: Vec<crate::entities::project::Model>,
        labels: Vec<crate::entities::label::Model>,
        sections: Vec<crate::entities::section::Model>,
        tasks: Vec<crate::entities::task::Model>,
    },
    SearchTasks(String), // Query for task search
    SearchResultsLoaded {
        query: String,
        results: Vec<crate::entities::task::Model>,
    },

    // Data refresh after task operations
    RefreshData,

    // UI operations
    ToggleSidebar,
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
        default_project_uuid: Option<Uuid>,
    },
    TaskEdit {
        task_uuid: Uuid,
        content: String,
        project_uuid: Uuid,
    },
    ProjectCreation,
    ProjectEdit {
        project_uuid: Uuid,
        name: String,
    },
    LabelCreation,
    LabelEdit {
        label_uuid: Uuid,
        name: String,
    },
    DeleteConfirmation {
        item_type: String,
        item_uuid: Uuid,
    },
    Error(String),
    Info(String),
    Help,
    Logs,
    TaskSearch,
}
