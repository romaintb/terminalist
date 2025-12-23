use crate::entities::section;
use crate::repositories::SectionRepository;
use crate::sync::SyncService;
use anyhow::Result;
use uuid::Uuid;

impl SyncService {
    /// Get all sections from local storage (fast)
    pub async fn get_sections(&self) -> Result<Vec<section::Model>> {
        let storage = self.storage.lock().await;
        SectionRepository::get_all(&storage.conn).await
    }

    /// Get sections for a project from local storage (fast)
    pub async fn get_sections_for_project(&self, project_uuid: &Uuid) -> Result<Vec<section::Model>> {
        let storage = self.storage.lock().await;
        SectionRepository::get_for_project(&storage.conn, project_uuid).await
    }
}
