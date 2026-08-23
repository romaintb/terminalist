use crate::sync::SyncStatus;
use uuid::Uuid;

/// Whether a task-list reload keeps the cursor on the same row or moves it to follow the
/// task that was selected before the reload.
///
/// This travels with each reload (as a field on [`Action::DataLoaded`]) rather than living as
/// mutable state on the app component, because the reload is handled by shared code
/// (`sync_component_data`/`TaskListComponent::update_data`) that serves every reload origin
/// and cannot infer the caller's intent on its own. A mutable "pending policy" flag would risk
/// a later reload from a different origin consuming a stale value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionPolicy {
    /// Leave `selected_index` exactly where it was. Use this for user-initiated reloads: after
    /// a task operation (`Action::RefreshData`), a sidebar navigation, or a debug-mode local
    /// refresh. The user is looking at a specific row and expects the cursor to stay there even
    /// if a different task now occupies it — e.g. pressing `t` to mark an overdue task due
    /// "today" must not drag the cursor along with it as it moves out of the Overdue section.
    KeepIndex,
    /// Re-anchor selection to the previously selected task's UUID, so the cursor follows that
    /// task even though its position in the list may have changed. Use this only for reloads
    /// triggered by data arriving out-of-band, i.e. a completed sync (manual `r` or the
    /// auto-sync timer): the user did not initiate the reload and may be navigating while it
    /// lands, so an index-based selection could silently jump to an unrelated task.
    FollowTask,
}

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
    CompleteTask(String),
    DeleteTask(String),
    CyclePriority(String),
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
    RestoreTask(String),

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
    InitialDataLoaded {
        projects: Vec<crate::entities::project::Model>,
        labels: Vec<crate::entities::label::Model>,
        sections: Vec<crate::entities::section::Model>,
        tasks: Vec<crate::entities::task::Model>,
    },
    DataLoaded {
        projects: Vec<crate::entities::project::Model>,
        labels: Vec<crate::entities::label::Model>,
        sections: Vec<crate::entities::section::Model>,
        tasks: Vec<crate::entities::task::Model>,
        selection_policy: SelectionPolicy,
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
