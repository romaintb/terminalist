//! Label repository for database operations.

use anyhow::Result;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

use crate::entities::label;

/// Repository for label-related database operations.
pub struct LabelRepository;

impl LabelRepository {
    /// Look up remote_id from local label UUID.
    pub async fn get_remote_id<C>(conn: &C, uuid: &Uuid) -> Result<String>
    where
        C: ConnectionTrait,
    {
        label::Entity::find()
            .filter(label::Column::Uuid.eq(*uuid))
            .one(conn)
            .await?
            .map(|l| l.remote_id)
            .ok_or_else(|| anyhow::anyhow!("Label not found: {}", uuid))
    }

    /// Get all labels ordered by order index.
    pub async fn get_all<C>(conn: &C) -> Result<Vec<label::Model>>
    where
        C: ConnectionTrait,
    {
        Ok(label::Entity::find().order_by_asc(label::Column::OrderIndex).all(conn).await?)
    }

    /// Get a single label by UUID.
    pub async fn get_by_id<C>(conn: &C, uuid: &Uuid) -> Result<Option<label::Model>>
    where
        C: ConnectionTrait,
    {
        Ok(label::Entity::find().filter(label::Column::Uuid.eq(*uuid)).one(conn).await?)
    }

    /// Get a single label by name.
    pub async fn get_by_name<C>(conn: &C, name: &str) -> Result<Option<label::Model>>
    where
        C: ConnectionTrait,
    {
        Ok(label::Entity::find().filter(label::Column::Name.eq(name)).one(conn).await?)
    }

    /// Update a label in the database.
    pub async fn update<C>(conn: &C, label: label::ActiveModel) -> Result<label::Model>
    where
        C: ConnectionTrait,
    {
        use sea_orm::ActiveModelTrait;
        Ok(label.update(conn).await?)
    }
}
