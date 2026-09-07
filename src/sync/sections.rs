use crate::entities::section;
use crate::repositories::SectionRepository;
use crate::sync::SyncService;
use anyhow::Result;

impl SyncService {
    /// Get all sections from local storage (fast)
    pub async fn get_sections(&self) -> Result<Vec<section::Model>> {
        let storage = self.storage.lock().await;
        SectionRepository::get_all(&storage.conn).await
    }
}
