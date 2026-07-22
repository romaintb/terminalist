use crate::{entities::task, sync::SyncStatus};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct NavigationCounts {
    pub today: usize,
    pub tomorrow: usize,
    pub upcoming: usize,
    pub trash: usize,
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
    Agenda,   // Local smart view derived from Today
    Tomorrow, // Tomorrow view (special view)
    Upcoming, // Upcoming view (tasks with future due dates)
    Trash,
    Label(Uuid),
    Project(Uuid),
}

#[derive(Debug, Clone)]
pub enum Action {
    // Navigation
    NavigateToSidebar(SidebarSelection),
    NextTask,
    PreviousTask,

    // Task operations
    ToggleTasks(Vec<(Uuid, bool)>),
    DeleteTask(Uuid),
    CyclePriority(Uuid),
    SetTaskDueToday(Uuid),
    SetTaskDueTomorrow(Uuid),
    SetTaskDueNextWeek(Uuid),
    SetTaskDueWeekEnd(Uuid),
    SetTaskDueTime {
        task_uuid: Uuid,
        due_datetime: String,
    },
    SetTasksDueDate {
        task_ids: Vec<Uuid>,
        due_date: TaskDueDate,
    },
    CreateTask {
        content: String,
        project_uuid: Option<Uuid>,
        due_date: Option<String>,
        label_uuid: Option<Uuid>,
    },
    EditTask {
        task_uuid: Uuid,
        content: String,
    },
    RestoreTask(Uuid),
    EmptyTrash,

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
    DataLoaded(Box<crate::ui::core::ViewSnapshot>),
    DataLoadFailed {
        generation: u64,
        selection: SidebarSelection,
        message: String,
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

impl Action {
    /// Build the shared completion-toggle action used by every task view.
    ///
    /// The boolean records whether the task should be restored rather than
    /// completed. Deleted tasks use the same restore path as completed tasks.
    pub fn toggle_tasks<'a>(tasks: impl IntoIterator<Item = &'a task::Model>) -> Self {
        let tasks = tasks
            .into_iter()
            .map(|task| (task.uuid, task.is_deleted || task.is_completed))
            .collect::<Vec<_>>();

        if tasks.is_empty() {
            Self::None
        } else {
            Self::ToggleTasks(tasks)
        }
    }

    pub fn toggle_task(task: &task::Model) -> Self {
        Self::toggle_tasks([task])
    }
}

#[derive(Debug, Clone)]
pub enum DialogType {
    TaskCreation {
        default_project_uuid: Option<Uuid>,
        default_due_date: Option<String>,
        default_label_uuid: Option<Uuid>,
    },
    TaskEdit {
        task_uuid: Uuid,
        content: String,
        project_uuid: Uuid,
    },
    TaskTime {
        task_uuid: Uuid,
        current_time: Option<String>,
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
    EmptyTrashConfirmation {
        count: usize,
    },
    Error(String),
    Info(String),
    Help,
    Logs,
    TaskSearch,
}
