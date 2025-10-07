//! Project repository for database operations.

use anyhow::Result;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

use crate::entities::project;

/// Repository for project-related database operations.
pub struct ProjectRepository;

impl ProjectRepository {
    /// Look up remote_id from local project UUID.
    pub async fn get_remote_id<C>(conn: &C, uuid: &Uuid) -> Result<String>
    where
        C: ConnectionTrait,
    {
        project::Entity::find()
            .filter(project::Column::Uuid.eq(*uuid))
            .one(conn)
            .await?
            .map(|p| p.remote_id)
            .ok_or_else(|| anyhow::anyhow!("Project not found: {}", uuid))
    }

    /// Get all projects ordered by order index.
    pub async fn get_all<C>(conn: &C) -> Result<Vec<project::Model>>
    where
        C: ConnectionTrait,
    {
        Ok(project::Entity::find()
            .order_by_asc(project::Column::OrderIndex)
            .all(conn)
            .await?)
    }

    /// Get a single project by UUID.
    pub async fn get_by_id<C>(conn: &C, uuid: &Uuid) -> Result<Option<project::Model>>
    where
        C: ConnectionTrait,
    {
        Ok(project::Entity::find()
            .filter(project::Column::Uuid.eq(*uuid))
            .one(conn)
            .await?)
    }

    /// Get a single project by remote_id.
    pub async fn get_by_remote_id<C>(conn: &C, remote_id: &str) -> Result<Option<project::Model>>
    where
        C: ConnectionTrait,
    {
        Ok(project::Entity::find()
            .filter(project::Column::RemoteId.eq(remote_id))
            .one(conn)
            .await?)
    }

    /// Update a project in the database.
    pub async fn update<C>(conn: &C, project: project::ActiveModel) -> Result<project::Model>
    where
        C: ConnectionTrait,
    {
        use sea_orm::ActiveModelTrait;
        Ok(project.update(conn).await?)
    }

    /// Delete a project from the database.
    pub async fn delete<C>(conn: &C, project: project::Model) -> Result<()>
    where
        C: ConnectionTrait,
    {
        use sea_orm::ModelTrait;
        project.delete(conn).await?;
        Ok(())
    }
}
