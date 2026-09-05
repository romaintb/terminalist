//! The background operations the UI can ask for.
//!
//! Every operation is a value carrying exactly the arguments it needs, so a caller
//! cannot ask for one that does not exist, forget an argument, or hand over user text
//! that has to survive a round trip through a delimiter.

use crate::constants::*;
use crate::sync::SyncService;
use crate::utils::datetime;
use uuid::Uuid;

/// The due dates a key binding can set. Todoist takes a plain date string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Due {
    Today,
    Tomorrow,
    NextWeek,
    Weekend,
}

impl Due {
    /// The date string Todoist expects for this shorthand.
    #[must_use]
    pub fn date(self) -> String {
        let today = chrono::Local::now().date_naive();
        match self {
            Due::Today => datetime::format_today(),
            Due::Tomorrow => datetime::format_date_with_offset(1),
            Due::NextWeek => datetime::format_ymd(datetime::next_weekday(today, chrono::Weekday::Mon)),
            Due::Weekend => datetime::format_ymd(datetime::next_weekday(today, chrono::Weekday::Sat)),
        }
    }

    /// What to tell the user once the due date lands.
    #[must_use]
    pub fn success(self) -> &'static str {
        match self {
            Due::Today => SUCCESS_TASK_DUE_TODAY,
            Due::Tomorrow => SUCCESS_TASK_DUE_TOMORROW,
            Due::NextWeek => SUCCESS_TASK_DUE_MONDAY,
            Due::Weekend => SUCCESS_TASK_DUE_SATURDAY,
        }
    }
}

/// A unit of work to run off the UI thread.
#[derive(Debug, Clone)]
pub enum Operation {
    CompleteTask(Uuid),
    DeleteTask(Uuid),
    RestoreTask(Uuid),
    CyclePriority { task: Uuid, priority: i32 },
    SetDue { task: Uuid, when: Due },
    CreateTask { content: String, project: Option<Uuid> },
    EditTask { task: Uuid, content: String },
    CreateProject { name: String, parent: Option<Uuid> },
    EditProject { project: Uuid, name: String },
    DeleteProject(Uuid),
    CreateLabel { name: String },
    EditLabel { label: Uuid, name: String },
    DeleteLabel(Uuid),
}

impl Operation {
    /// What the task manager lists this operation as, and what gets logged when it
    /// starts. Names the subject rather than the user's text, which can be long.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Operation::CompleteTask(task) => format!("Complete task: {task}"),
            Operation::DeleteTask(task) => format!("Delete task: {task}"),
            Operation::RestoreTask(task) => format!("Restore task: {task}"),
            Operation::CyclePriority { task, priority } => format!("Cycle priority: {task} -> P{priority}"),
            Operation::SetDue { task, when } => format!("Set task due {when:?}: {task}"),
            Operation::CreateTask { project, .. } => match project {
                Some(project) => format!("Create task: in project {project}"),
                None => "Create task: in inbox".to_string(),
            },
            Operation::EditTask { task, .. } => format!("Edit task: {task}"),
            Operation::CreateProject { parent, .. } => match parent {
                Some(parent) => format!("Create project: under {parent}"),
                None => "Create project: at root".to_string(),
            },
            Operation::EditProject { project, .. } => format!("Edit project: {project}"),
            Operation::DeleteProject(project) => format!("Delete project: {project}"),
            Operation::CreateLabel { name } => format!("Create label: {name}"),
            Operation::EditLabel { label, .. } => format!("Edit label: {label}"),
            Operation::DeleteLabel(label) => format!("Delete label: {label}"),
        }
    }

    /// Runs the operation, reporting either the message to show on success or the one
    /// to show on failure.
    pub async fn run(self, sync: SyncService) -> Result<String, String> {
        fn done(success: &str, subject: impl std::fmt::Display) -> Result<String, String> {
            Ok(format!("{success}: {subject}"))
        }
        fn failed(error: &str, e: impl std::fmt::Display) -> Result<String, String> {
            Err(format!("{error}: {e}"))
        }

        match self {
            Operation::CompleteTask(task) => match sync.complete_task(&task).await {
                Ok(()) => done(SUCCESS_TASK_COMPLETED, task),
                Err(e) => failed(ERROR_TASK_COMPLETION_FAILED, e),
            },
            Operation::DeleteTask(task) => match sync.delete_task(&task).await {
                Ok(()) => done(SUCCESS_TASK_DELETED, task),
                Err(e) => failed(ERROR_TASK_DELETE_FAILED, e),
            },
            Operation::RestoreTask(task) => match sync.restore_task(&task).await {
                Ok(()) => done(SUCCESS_TASK_RESTORED, task),
                Err(e) => failed(ERROR_TASK_RESTORE_FAILED, e),
            },
            Operation::CyclePriority { task, priority } => match sync.update_task_priority(&task, priority).await {
                Ok(()) => Ok(format!("{SUCCESS_TASK_PRIORITY_UPDATED}{priority}: {task}")),
                Err(e) => failed(ERROR_TASK_PRIORITY_FAILED, e),
            },
            Operation::SetDue { task, when } => match sync.update_task_due_date(&task, Some(&when.date())).await {
                Ok(()) => done(when.success(), task),
                Err(e) => failed(ERROR_TASK_DUE_DATE_FAILED, e),
            },
            Operation::CreateTask { content, project } => {
                let landed = if project.is_some() {
                    SUCCESS_TASK_CREATED_PROJECT
                } else {
                    SUCCESS_TASK_CREATED_INBOX
                };
                match sync.create_task(&content, project).await {
                    Ok(()) => done(landed, content),
                    Err(e) => failed(ERROR_TASK_CREATE_FAILED, e),
                }
            }
            Operation::EditTask { task, content } => match sync.update_task_content(&task, &content).await {
                Ok(()) => done(SUCCESS_TASK_UPDATED, task),
                Err(e) => failed(ERROR_TASK_UPDATE_FAILED, e),
            },
            Operation::CreateProject { name, parent } => {
                let landed = if parent.is_some() {
                    SUCCESS_PROJECT_CREATED_PARENT
                } else {
                    SUCCESS_PROJECT_CREATED_ROOT
                };
                match sync.create_project(&name, parent).await {
                    Ok(()) => done(landed, name),
                    Err(e) => failed(ERROR_PROJECT_CREATE_FAILED, e),
                }
            }
            Operation::EditProject { project, name } => match sync.update_project_content(&project, &name).await {
                Ok(()) => done(SUCCESS_PROJECT_UPDATED, project),
                Err(e) => failed(ERROR_PROJECT_UPDATE_FAILED, e),
            },
            Operation::DeleteProject(project) => match sync.delete_project(&project).await {
                Ok(()) => done(SUCCESS_PROJECT_DELETED, project),
                Err(e) => failed(ERROR_PROJECT_DELETE_FAILED, e),
            },
            Operation::CreateLabel { name } => match sync.create_label(&name).await {
                Ok(()) => done(SUCCESS_LABEL_CREATED, name),
                Err(e) => failed(ERROR_LABEL_CREATE_FAILED, e),
            },
            Operation::EditLabel { label, name } => match sync.update_label_content(&label, &name).await {
                Ok(()) => done(SUCCESS_LABEL_UPDATED, label),
                Err(e) => failed(ERROR_LABEL_UPDATE_FAILED, e),
            },
            Operation::DeleteLabel(label) => match sync.delete_label(&label).await {
                Ok(()) => done(SUCCESS_LABEL_DELETED, label),
                Err(e) => failed(ERROR_LABEL_DELETE_FAILED, e),
            },
        }
    }
}
