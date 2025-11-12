//! Backend repository for database operations.

use anyhow::Result;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::entities::backend;

/// Repository for backend-related database operations.
pub struct BackendRepository;

impl BackendRepository {
    /// Get the first backend (for single-backend scenarios).
    pub async fn get_first<C>(conn: &C) -> Result<Option<backend::Model>>
    where
        C: ConnectionTrait,
    {
        Ok(backend::Entity::find().one(conn).await?)
    }

    /// Get a backend by UUID.
    pub async fn get_by_uuid<C>(conn: &C, uuid: &Uuid) -> Result<Option<backend::Model>>
    where
        C: ConnectionTrait,
    {
        Ok(backend::Entity::find()
            .filter(backend::Column::Uuid.eq(*uuid))
            .one(conn)
            .await?)
    }

    /// Get all backends.
    pub async fn get_all<C>(conn: &C) -> Result<Vec<backend::Model>>
    where
        C: ConnectionTrait,
    {
        Ok(backend::Entity::find().all(conn).await?)
    }

    /// Get all enabled backends.
    pub async fn get_enabled<C>(conn: &C) -> Result<Vec<backend::Model>>
    where
        C: ConnectionTrait,
    {
        Ok(backend::Entity::find()
            .filter(backend::Column::IsEnabled.eq(true))
            .all(conn)
            .await?)
    }

    /// Create a new backend.
    pub async fn create<C>(conn: &C, backend: backend::ActiveModel) -> Result<backend::Model>
    where
        C: ConnectionTrait,
    {
        Ok(backend.insert(conn).await?)
    }

    /// Update an existing backend.
    pub async fn update<C>(conn: &C, backend: backend::ActiveModel) -> Result<backend::Model>
    where
        C: ConnectionTrait,
    {
        Ok(backend.update(conn).await?)
    }

    /// Delete a backend by UUID.
    pub async fn delete<C>(conn: &C, uuid: &Uuid) -> Result<()>
    where
        C: ConnectionTrait,
    {
        if let Some(backend) = Self::get_by_uuid(conn, uuid).await? {
            use sea_orm::ModelTrait;
            backend.delete(conn).await?;
        }
        Ok(())
    }
}
