//! Section repository for database operations.

use anyhow::Result;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

use crate::entities::section;

/// Repository for section-related database operations.
pub struct SectionRepository;

impl SectionRepository {
    /// Get all sections ordered by order index.
    pub async fn get_all<C>(conn: &C) -> Result<Vec<section::Model>>
    where
        C: ConnectionTrait,
    {
        Ok(section::Entity::find()
            .order_by_asc(section::Column::OrderIndex)
            .all(conn)
            .await?)
    }

    /// Get sections for a specific project.
    pub async fn get_for_project<C>(conn: &C, project_uuid: &Uuid) -> Result<Vec<section::Model>>
    where
        C: ConnectionTrait,
    {
        Ok(section::Entity::find()
            .filter(section::Column::ProjectUuid.eq(*project_uuid))
            .order_by_asc(section::Column::OrderIndex)
            .all(conn)
            .await?)
    }

    /// Get a single section by UUID.
    pub async fn get_by_id<C>(conn: &C, uuid: &Uuid) -> Result<Option<section::Model>>
    where
        C: ConnectionTrait,
    {
        Ok(section::Entity::find()
            .filter(section::Column::Uuid.eq(*uuid))
            .one(conn)
            .await?)
    }

    /// Get a single section by remote_id.
    pub async fn get_by_remote_id<C>(conn: &C, remote_id: &str) -> Result<Option<section::Model>>
    where
        C: ConnectionTrait,
    {
        Ok(section::Entity::find()
            .filter(section::Column::RemoteId.eq(remote_id))
            .one(conn)
            .await?)
    }

    /// Look up remote_id from local section UUID.
    pub async fn get_remote_id<C>(conn: &C, uuid: &Uuid) -> Result<Option<String>>
    where
        C: ConnectionTrait,
    {
        Ok(section::Entity::find()
            .filter(section::Column::Uuid.eq(*uuid))
            .one(conn)
            .await?
            .map(|s| s.remote_id))
    }
}
