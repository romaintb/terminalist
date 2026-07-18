use super::actions::TaskDueDate;
use crate::sync::SyncService;
use crate::utils::datetime;
use anyhow::{Context, Result};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum TaskOperation {
    Complete(Uuid),
    Delete(Uuid),
    Restore(Uuid),
    CyclePriority {
        task_uuid: Uuid,
        priority: i32,
    },
    SetDueDate {
        task_uuid: Uuid,
        due_date: TaskDueDate,
    },
    Create {
        content: String,
        project_uuid: Option<Uuid>,
    },
    Edit {
        task_uuid: Uuid,
        content: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectOperation {
    Create { name: String, parent_uuid: Option<Uuid> },
    Edit { project_uuid: Uuid, name: String },
    Delete(Uuid),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LabelOperation {
    Create { name: String },
    Edit { label_uuid: Uuid, name: String },
    Delete(Uuid),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operation {
    Task(TaskOperation),
    Project(ProjectOperation),
    Label(LabelOperation),
}

impl Operation {
    pub fn description(&self) -> String {
        match self {
            Self::Task(TaskOperation::Complete(_)) => "Complete task",
            Self::Task(TaskOperation::Delete(_)) => "Delete task",
            Self::Task(TaskOperation::Restore(_)) => "Restore task",
            Self::Task(TaskOperation::CyclePriority { .. }) => "Cycle task priority",
            Self::Task(TaskOperation::SetDueDate { due_date, .. }) => match due_date {
                TaskDueDate::None => "Unschedule task",
                TaskDueDate::Today => "Schedule task for today",
                TaskDueDate::Tomorrow => "Schedule task for tomorrow",
                TaskDueDate::NextWeek => "Schedule task for next week",
                TaskDueDate::Weekend => "Schedule task for the weekend",
            },
            Self::Task(TaskOperation::Create { .. }) => "Create task",
            Self::Task(TaskOperation::Edit { .. }) => "Edit task",
            Self::Project(ProjectOperation::Create { .. }) => "Create project",
            Self::Project(ProjectOperation::Edit { .. }) => "Edit project",
            Self::Project(ProjectOperation::Delete(_)) => "Delete project",
            Self::Label(LabelOperation::Create { .. }) => "Create label",
            Self::Label(LabelOperation::Edit { .. }) => "Edit label",
            Self::Label(LabelOperation::Delete(_)) => "Delete label",
        }
        .to_string()
    }

    pub async fn execute(self, sync_service: &SyncService) -> Result<String> {
        let description = self.description();
        match self {
            Self::Task(TaskOperation::Complete(task_uuid)) => {
                sync_service.complete_task(&task_uuid).await.context(description.clone())?
            }
            Self::Task(TaskOperation::Delete(task_uuid)) => {
                sync_service.delete_task(&task_uuid).await.context(description.clone())?
            }
            Self::Task(TaskOperation::Restore(task_uuid)) => {
                sync_service.restore_task(&task_uuid).await.context(description.clone())?
            }
            Self::Task(TaskOperation::CyclePriority { task_uuid, priority }) => sync_service
                .update_task_priority(&task_uuid, priority)
                .await
                .context(description.clone())?,
            Self::Task(TaskOperation::SetDueDate { task_uuid, due_date }) => {
                let due_date = match due_date {
                    TaskDueDate::None => None,
                    TaskDueDate::Today => Some(datetime::format_today()),
                    TaskDueDate::Tomorrow => Some(datetime::format_date_with_offset(1)),
                    TaskDueDate::NextWeek => {
                        let today = chrono::Local::now().date_naive();
                        Some(datetime::format_ymd(datetime::next_weekday(
                            today,
                            chrono::Weekday::Mon,
                        )))
                    }
                    TaskDueDate::Weekend => {
                        let today = chrono::Local::now().date_naive();
                        Some(datetime::format_ymd(datetime::next_weekday(
                            today,
                            chrono::Weekday::Sat,
                        )))
                    }
                };
                sync_service
                    .update_task_due_date(&task_uuid, due_date.as_deref())
                    .await
                    .context(description.clone())?
            }
            Self::Task(TaskOperation::Create { content, project_uuid }) => sync_service
                .create_task(&content, project_uuid)
                .await
                .context(description.clone())?,
            Self::Task(TaskOperation::Edit { task_uuid, content }) => sync_service
                .update_task_content(&task_uuid, &content)
                .await
                .context(description.clone())?,
            Self::Project(ProjectOperation::Create { name, parent_uuid }) => sync_service
                .create_project(&name, parent_uuid)
                .await
                .context(description.clone())?,
            Self::Project(ProjectOperation::Edit { project_uuid, name }) => sync_service
                .update_project_content(&project_uuid, &name)
                .await
                .context(description.clone())?,
            Self::Project(ProjectOperation::Delete(project_uuid)) => {
                sync_service.delete_project(&project_uuid).await.context(description.clone())?
            }
            Self::Label(LabelOperation::Create { name }) => {
                sync_service.create_label(&name).await.context(description.clone())?
            }
            Self::Label(LabelOperation::Edit { label_uuid, name }) => sync_service
                .update_label_content(&label_uuid, &name)
                .await
                .context(description.clone())?,
            Self::Label(LabelOperation::Delete(label_uuid)) => {
                sync_service.delete_label(&label_uuid).await.context(description.clone())?
            }
        }
        Ok(description)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_operations_preserve_delimiter_content() {
        let operation = Operation::Task(TaskOperation::Create {
            content: "Review A|B: preserve this exactly".to_string(),
            project_uuid: Some(Uuid::new_v4()),
        });

        match operation {
            Operation::Task(TaskOperation::Create { content, .. }) => {
                assert_eq!(content, "Review A|B: preserve this exactly");
            }
            _ => panic!("expected a typed task creation operation"),
        }
    }
}
