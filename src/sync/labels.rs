use crate::entities::label;
use crate::repositories::LabelRepository;
use crate::sync::SyncService;
use anyhow::Result;
use log::info;
use sea_orm::{ActiveValue, EntityTrait, IntoActiveModel};
use uuid::Uuid;

impl SyncService {
    /// Get all labels from local storage (fast)
    pub async fn get_labels(&self) -> Result<Vec<label::Model>> {
        let storage = self.storage.lock().await;
        LabelRepository::get_all(&storage.conn).await
    }

    /// Creates a new label via the remote backend and stores it locally.
    ///
    /// This method creates a label remotely and immediately stores it in local storage
    /// for instant UI updates. The label will be available in the UI without requiring
    /// a full sync operation.
    ///
    /// # Arguments
    /// * `name` - The name of the new label
    ///
    /// # Errors
    /// Returns an error if the backend call fails or local storage update fails
    pub async fn create_label(&self, name: &str) -> Result<()> {
        info!("Backend: Creating label '{}'", name);

        // Create label via backend using the CreateLabelArgs structure
        let label_args = crate::backend::CreateLabelArgs {
            name: name.to_string(),
            is_favorite: None,
        };
        let api_label = self
            .get_backend()
            .await?
            .create_label(label_args)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        // Store the created label in local database immediately for UI refresh
        info!("Storage: Storing new label locally with ID {}", api_label.remote_id);
        let storage = self.storage.lock().await;

        let local_label = label::ActiveModel {
            uuid: ActiveValue::Set(Uuid::new_v4()),
            backend_uuid: ActiveValue::Set(self.backend_uuid),
            remote_id: ActiveValue::Set(api_label.remote_id),
            name: ActiveValue::Set(api_label.name),
            order_index: ActiveValue::Set(api_label.order_index),
            is_favorite: ActiveValue::Set(api_label.is_favorite),
        };

        use sea_orm::sea_query::OnConflict;
        let mut insert = label::Entity::insert(local_label);
        insert = insert.on_conflict(
            OnConflict::columns([label::Column::BackendUuid, label::Column::RemoteId])
                .update_columns([label::Column::Name, label::Column::OrderIndex, label::Column::IsFavorite])
                .to_owned(),
        );
        insert.exec(&storage.conn).await?;

        Ok(())
    }

    /// Update label content (name only for now)
    pub async fn update_label_content(&self, label_uuid: &Uuid, name: &str) -> Result<()> {
        info!("Backend: Updating label name for UUID {} to '{}'", label_uuid, name);

        // Look up the label's remote_id for backend call
        let remote_id = self.get_label_remote_id(label_uuid).await?;

        // Update label via backend using the UpdateLabelArgs structure
        let label_args = crate::backend::UpdateLabelArgs {
            name: Some(name.to_string()),
            is_favorite: None,
        };
        let _label = self
            .get_backend()
            .await?
            .update_label(&remote_id, label_args)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        // Update local storage immediately after successful backend call
        info!(
            "Storage: Updating local label name for UUID {} to '{}'",
            label_uuid, name
        );
        let storage = self.storage.lock().await;

        if let Some(label) = LabelRepository::get_by_id(&storage.conn, label_uuid).await? {
            let mut active_model: label::ActiveModel = label.into_active_model();
            active_model.name = ActiveValue::Set(name.to_string());
            LabelRepository::update(&storage.conn, active_model).await?;
        }

        Ok(())
    }

    /// Delete a label
    pub async fn delete_label(&self, label_uuid: &Uuid) -> Result<()> {
        // Look up the label's remote_id for backend call
        let remote_id = self.get_label_remote_id(label_uuid).await?;

        // Delete label via backend
        self.get_backend()
            .await?
            .delete_label(&remote_id)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        // Note: Local storage deletion will be handled by the next sync
        Ok(())
    }
}
