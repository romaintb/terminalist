use crate::entities::project;
use crate::repositories::ProjectRepository;
use crate::sync::SyncService;
use anyhow::Result;
use log::warn;
use sea_orm::{ActiveValue, EntityTrait, IntoActiveModel};
use uuid::Uuid;

impl SyncService {
    /// Retrieves all projects from local storage.
    ///
    /// This method provides fast access to cached project data without making backend calls.
    /// Projects are sorted and ready for display in the UI.
    ///
    /// # Returns
    /// A vector of `project::Model` objects representing all available projects
    ///
    /// # Note
    /// As of 2025, Todoist allows free plan users to create more than 5 projects via the backend,
    /// but the GET /projects backend endpoint will only return the first 5 projects for free users.
    ///
    /// # Errors
    /// Returns an error if local storage access fails
    pub async fn get_projects(&self) -> Result<Vec<project::Model>> {
        let storage = self.storage.lock().await;
        ProjectRepository::get_all(&storage.conn).await
    }

    /// Creates a new project via the remote backend and stores it locally.
    ///
    /// This method creates a project remotely and immediately stores it in local storage
    /// for instant UI updates. The project will be available in the UI without requiring
    /// a full sync operation.
    ///
    /// # Arguments
    /// * `name` - The name of the new project
    /// * `parent_uuid` - Optional parent project UUID for creating sub-projects
    ///
    /// # Note
    /// As of 2025, Todoist allows free plan users to create more than 5 projects via the backend,
    /// but the GET /projects backend endpoint will only return the first 5 projects for free users.
    ///
    /// # Errors
    /// Returns an error if the backend call fails or local storage update fails
    pub async fn create_project(&self, name: &str, parent_uuid: Option<Uuid>) -> Result<()> {
        // Look up remote_id for parent project if provided
        let remote_parent_id = if let Some(uuid) = parent_uuid {
            Some(self.get_project_remote_id(&uuid).await?)
        } else {
            None
        };

        // Create project via backend using backend CreateProjectArgs
        let project_args = crate::backend::CreateProjectArgs {
            name: name.to_string(),
            parent_remote_id: remote_parent_id,
            is_favorite: None,
        };
        let backend_project = self
            .get_backend()
            .await?
            .create_project(project_args)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        // Store the created project in local database immediately for UI refresh
        let storage = self.storage.lock().await;

        // Upsert the project
        let local_project = project::ActiveModel {
            uuid: ActiveValue::Set(Uuid::new_v4()),
            backend_uuid: ActiveValue::Set(self.backend_uuid),
            remote_id: ActiveValue::Set(backend_project.remote_id),
            name: ActiveValue::Set(backend_project.name),
            is_favorite: ActiveValue::Set(backend_project.is_favorite),
            is_inbox_project: ActiveValue::Set(backend_project.is_inbox),
            order_index: ActiveValue::Set(backend_project.order_index),
            parent_uuid: ActiveValue::Set(parent_uuid),
        };

        use sea_orm::sea_query::OnConflict;
        let mut insert = project::Entity::insert(local_project);
        insert = insert.on_conflict(
            OnConflict::columns([project::Column::BackendUuid, project::Column::RemoteId])
                .update_columns([
                    project::Column::Name,
                    project::Column::IsFavorite,
                    project::Column::IsInboxProject,
                    project::Column::OrderIndex,
                ])
                .to_owned(),
        );
        insert.exec(&storage.conn).await?;

        Ok(())
    }

    /// Update project content (name only for now)
    pub async fn update_project_content(&self, project_uuid: &Uuid, name: &str) -> Result<()> {
        // Look up the project's remote_id for backend call
        let remote_id = self.get_project_remote_id(project_uuid).await?;

        // Update project via backend using the UpdateProjectArgs structure
        let project_args = crate::backend::UpdateProjectArgs {
            name: Some(name.to_string()),
            is_favorite: None,
        };
        let _project = self
            .get_backend()
            .await?
            .update_project(&remote_id, project_args)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        // Update local storage immediately after successful backend call
        let storage = self.storage.lock().await;

        if let Some(project) = ProjectRepository::get_by_id(&storage.conn, project_uuid).await? {
            let mut active_model: project::ActiveModel = project.into_active_model();
            active_model.name = ActiveValue::Set(name.to_string());
            ProjectRepository::update(&storage.conn, active_model).await?;
        } else {
            warn!(
                "Local project with UUID {} not found after successful backend update.",
                project_uuid
            );
        }

        Ok(())
    }

    /// Delete a project
    pub async fn delete_project(&self, project_uuid: &Uuid) -> Result<()> {
        // Look up the project's remote_id for backend call
        let remote_id = self.get_project_remote_id(project_uuid).await?;

        // Delete project via backend
        self.get_backend()
            .await?
            .delete_project(&remote_id)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        // Remove from local storage
        let storage = self.storage.lock().await;

        if let Some(project) = ProjectRepository::get_by_id(&storage.conn, project_uuid).await? {
            ProjectRepository::delete(&storage.conn, project).await?;
        } else {
            warn!(
                "Local project with UUID {} not found after successful backend deletion.",
                project_uuid
            );
        }

        Ok(())
    }
}
