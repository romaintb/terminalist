use anyhow::Result;
use sea_orm::{prelude::*, ActiveValue, IntoActiveModel, QueryOrder};

use crate::entities::project;
use crate::todoist::{Project as TodoistProject, ProjectDisplay};

use super::LocalStorage;

impl From<project::Model> for ProjectDisplay {
    fn from(model: project::Model) -> Self {
        Self {
            id: model.uuid,
            name: model.name,
            color: model.color,
            is_favorite: model.is_favorite,
            parent_id: model.parent_uuid,
            is_inbox_project: model.is_inbox_project,
        }
    }
}

impl LocalStorage {
    /// Store a project in the database
    pub async fn store_project(&self, project_data: &TodoistProject) -> Result<()> {
        let model = project::ActiveModel {
            uuid: ActiveValue::Set(Uuid::new_v4().to_string()),
            remote_id: ActiveValue::Set(project_data.id.clone()),
            name: ActiveValue::Set(project_data.name.clone()),
            color: ActiveValue::Set(project_data.color.clone()),
            is_favorite: ActiveValue::Set(project_data.is_favorite),
            is_inbox_project: ActiveValue::Set(project_data.is_inbox_project),
            order_index: ActiveValue::Set(project_data.order),
            parent_uuid: ActiveValue::Set(None),
        };

        project::Entity::insert(model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(project::Column::RemoteId)
                    .update_columns([
                        project::Column::Name,
                        project::Column::Color,
                        project::Column::IsFavorite,
                        project::Column::IsInboxProject,
                        project::Column::OrderIndex,
                    ])
                    .to_owned(),
            )
            .exec(&self.conn)
            .await?;

        Ok(())
    }

    /// Get all projects ordered by order_index
    pub async fn get_all_projects(&self) -> Result<Vec<project::Model>> {
        let projects = project::Entity::find()
            .order_by_asc(project::Column::OrderIndex)
            .all(&self.conn)
            .await?;

        Ok(projects)
    }

    /// Get all projects as ProjectDisplay (for UI)
    pub async fn get_projects(&self) -> Result<Vec<ProjectDisplay>> {
        let projects = self.get_all_projects().await?;
        Ok(projects.into_iter().map(|p| p.into()).collect())
    }

    /// Get a project by its remote ID
    pub async fn get_project_by_remote_id(&self, remote_id: &str) -> Result<Option<project::Model>> {
        let project = project::Entity::find()
            .filter(project::Column::RemoteId.eq(remote_id))
            .one(&self.conn)
            .await?;

        Ok(project)
    }

    /// Get a project by its UUID
    pub async fn get_project_by_uuid(&self, uuid: &str) -> Result<Option<project::Model>> {
        let project = project::Entity::find()
            .filter(project::Column::Uuid.eq(uuid))
            .one(&self.conn)
            .await?;

        Ok(project)
    }

    /// Delete all projects
    pub async fn delete_all_projects(&self) -> Result<()> {
        project::Entity::delete_many().exec(&self.conn).await?;
        Ok(())
    }

    /// Update project parent relationships after all projects are stored
    pub async fn update_project_parents(&self, projects: &[TodoistProject]) -> Result<()> {
        for project_data in projects {
            if let Some(parent_id) = &project_data.parent_id {
                // Find the parent project's UUID
                if let Some(parent) = self.get_project_by_remote_id(parent_id).await? {
                    // Find the current project and update its parent_uuid
                    if let Some(current) = self.get_project_by_remote_id(&project_data.id).await? {
                        let mut active_model: project::ActiveModel = current.into_active_model();
                        active_model.parent_uuid = ActiveValue::Set(Some(parent.uuid));
                        active_model.update(&self.conn).await?;
                    }
                }
            }
        }

        Ok(())
    }
}
