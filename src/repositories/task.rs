//! Task repository for database operations.

use anyhow::Result;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait};
use uuid::Uuid;

use crate::entities::{task, task_label};

/// Repository for task-related database operations.
pub struct TaskRepository;

impl TaskRepository {
    /// Look up remote_id from local task UUID.
    pub async fn get_remote_id<C>(conn: &C, uuid: &Uuid) -> Result<String>
    where
        C: ConnectionTrait,
    {
        task::Entity::find()
            .filter(task::Column::Uuid.eq(*uuid))
            .one(conn)
            .await?
            .map(|t| t.remote_id)
            .ok_or_else(|| anyhow::anyhow!("Task not found: {}", uuid))
    }

    /// Get all tasks ordered by deletion status, completion status, and order index.
    pub async fn get_all<C>(conn: &C) -> Result<Vec<task::Model>>
    where
        C: ConnectionTrait,
    {
        Ok(task::Entity::find()
            .order_by_asc(task::Column::IsDeleted)
            .order_by_asc(task::Column::OrderIndex)
            .all(conn)
            .await?)
    }

    /// Get a single task by UUID.
    pub async fn get_by_id<C>(conn: &C, uuid: &Uuid) -> Result<Option<task::Model>>
    where
        C: ConnectionTrait,
    {
        Ok(task::Entity::find().filter(task::Column::Uuid.eq(*uuid)).one(conn).await?)
    }

    /// Get a single task by remote_id.
    pub async fn get_by_remote_id<C>(conn: &C, remote_id: &str) -> Result<Option<task::Model>>
    where
        C: ConnectionTrait,
    {
        Ok(task::Entity::find()
            .filter(task::Column::RemoteId.eq(remote_id))
            .one(conn)
            .await?)
    }

    /// Get all tasks for a specific project.
    pub async fn get_for_project<C>(conn: &C, project_uuid: &Uuid) -> Result<Vec<task::Model>>
    where
        C: ConnectionTrait,
    {
        Ok(task::Entity::find()
            .filter(task::Column::ProjectUuid.eq(*project_uuid))
            .order_by_asc(task::Column::IsDeleted)
            .order_by_asc(task::Column::IsCompleted)
            .order_by_asc(task::Column::OrderIndex)
            .all(conn)
            .await?)
    }

    /// Search tasks by content or description.
    pub async fn search<C>(conn: &C, query: &str) -> Result<Vec<task::Model>>
    where
        C: ConnectionTrait,
    {
        use sea_orm::sea_query::Expr;
        Ok(task::Entity::find()
            .filter(
                Expr::col(task::Column::Content)
                    .like(format!("%{}%", query))
                    .or(Expr::col(task::Column::Description).like(format!("%{}%", query))),
            )
            .order_by_asc(task::Column::IsDeleted)
            .order_by_asc(task::Column::IsCompleted)
            .order_by_asc(task::Column::OrderIndex)
            .all(conn)
            .await?)
    }

    /// Get tasks with a specific label.
    pub async fn get_with_label<C>(conn: &C, label_uuid: Uuid) -> Result<Vec<task::Model>>
    where
        C: ConnectionTrait,
    {
        Ok(task::Entity::find()
            .filter(
                task::Column::Uuid.in_subquery(
                    task_label::Entity::find()
                        .filter(task_label::Column::LabelUuid.eq(label_uuid))
                        .select_only()
                        .column(task_label::Column::TaskUuid)
                        .into_query(),
                ),
            )
            .order_by_asc(task::Column::IsDeleted)
            .order_by_asc(task::Column::IsCompleted)
            .order_by_asc(task::Column::OrderIndex)
            .all(conn)
            .await?)
    }

    /// Get tasks for the "Today" view (overdue + today).
    pub async fn get_for_today<C>(conn: &C, today: &str) -> Result<Vec<task::Model>>
    where
        C: ConnectionTrait,
    {
        let overdue_tasks = task::Entity::overdue(today).all(conn).await?;
        let today_tasks = task::Entity::due_today(today).all(conn).await?;

        let mut result = overdue_tasks;
        result.extend(today_tasks);
        Ok(result)
    }

    /// Get tasks scheduled for tomorrow.
    pub async fn get_for_tomorrow<C>(conn: &C, tomorrow: &str) -> Result<Vec<task::Model>>
    where
        C: ConnectionTrait,
    {
        Ok(task::Entity::find()
            .filter(task::Column::DueDate.eq(tomorrow))
            .order_by_asc(task::Column::IsDeleted)
            .order_by_asc(task::Column::IsCompleted)
            .order_by_asc(task::Column::OrderIndex)
            .all(conn)
            .await?)
    }

    /// Get tasks for the "Upcoming" view (overdue + today + next 3 months).
    pub async fn get_for_upcoming<C>(conn: &C, today: &str, three_months_later: &str) -> Result<Vec<task::Model>>
    where
        C: ConnectionTrait,
    {
        let overdue_tasks = task::Entity::overdue(today).all(conn).await?;
        let today_tasks = task::Entity::due_today(today).all(conn).await?;
        let future_tasks = task::Entity::due_between(today, three_months_later).all(conn).await?;

        let mut result = overdue_tasks;
        result.extend(today_tasks);
        result.extend(future_tasks.into_iter().filter(|task| {
            if let Some(due_date) = &task.due_date {
                due_date != today
            } else {
                true
            }
        }));
        Ok(result)
    }

    /// Update a task in the database.
    pub async fn update<C>(conn: &C, task: task::ActiveModel) -> Result<task::Model>
    where
        C: ConnectionTrait,
    {
        use sea_orm::ActiveModelTrait;
        Ok(task.update(conn).await?)
    }

    /// Delete a task from the database.
    pub async fn delete<C>(conn: &C, task: task::Model) -> Result<()>
    where
        C: ConnectionTrait,
    {
        use sea_orm::ModelTrait;
        task.delete(conn).await?;
        Ok(())
    }
}
