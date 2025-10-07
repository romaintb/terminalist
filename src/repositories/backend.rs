//! Backend repository for database operations.

use anyhow::Result;
use sea_orm::{ConnectionTrait, EntityTrait};

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
}
