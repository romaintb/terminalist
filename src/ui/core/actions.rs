use crate::sync::SyncStatus;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct NavigationCounts {
    pub today: usize,
    pub tomorrow: usize,
    pub upcoming: usize,
    pub projects: HashMap<Uuid, usize>,
    pub labels: HashMap<Uuid, usize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaskDueDate {
    None,
    Today,
    Tomorrow,
    NextWeek,
    Weekend,
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
    CompleteTask(Uuid),
    ToggleTasks(Vec<(Uuid, bool)>),
    DeleteTask(Uuid),
    CyclePriority(Uuid),
    SetTaskDueToday(Uuid),
    SetTaskDueTomorrow(Uuid),
    SetTaskDueNextWeek(Uuid),
    SetTaskDueWeekEnd(Uuid),
    SetTasksDueDate {
        task_ids: Vec<Uuid>,
        due_date: TaskDueDate,
    },
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
    InitialDataLoaded {
        projects: Vec<crate::entities::project::Model>,
        labels: Vec<crate::entities::label::Model>,
        sections: Vec<crate::entities::section::Model>,
        tasks: Vec<crate::entities::task::Model>,
        all_tasks: Vec<crate::entities::task::Model>,
        navigation_counts: NavigationCounts,
    },
    DataLoaded {
        projects: Vec<crate::entities::project::Model>,
        labels: Vec<crate::entities::label::Model>,
        sections: Vec<crate::entities::section::Model>,
        tasks: Vec<crate::entities::task::Model>,
        all_tasks: Vec<crate::entities::task::Model>,
        navigation_counts: NavigationCounts,
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
    Consumed,

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
