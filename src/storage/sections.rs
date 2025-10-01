use anyhow::Result;
use sea_orm::{prelude::*, ActiveValue, QueryOrder};

use crate::entities::section;
use crate::todoist::{Section as TodoistSection, SectionDisplay};

use super::LocalStorage;

impl From<section::Model> for SectionDisplay {
    fn from(model: section::Model) -> Self {
        Self {
            id: model.uuid,
            name: model.name,
            project_id: model.project_uuid,
        }
    }
}

impl LocalStorage {
    /// Store a section in the database
    pub async fn store_section(&self, section_data: &TodoistSection, project_uuid: &str) -> Result<()> {
        let model = section::ActiveModel {
            uuid: ActiveValue::Set(Uuid::new_v4().to_string()),
            remote_id: ActiveValue::Set(section_data.id.clone()),
            name: ActiveValue::Set(section_data.name.clone()),
            project_uuid: ActiveValue::Set(project_uuid.to_string()),
            order_index: ActiveValue::Set(section_data.order),
        };

        section::Entity::insert(model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(section::Column::RemoteId)
                    .update_columns([
                        section::Column::Name,
                        section::Column::ProjectUuid,
                        section::Column::OrderIndex,
                    ])
                    .to_owned(),
            )
            .exec(&self.conn)
            .await?;

        Ok(())
    }

    /// Get all sections for a project
    pub async fn get_sections_by_project_uuid(&self, project_uuid: &str) -> Result<Vec<section::Model>> {
        let sections = section::Entity::find()
            .filter(section::Column::ProjectUuid.eq(project_uuid))
            .order_by_asc(section::Column::OrderIndex)
            .all(&self.conn)
            .await?;

        Ok(sections)
    }

    /// Get a section by its remote ID
    pub async fn get_section_by_remote_id(&self, remote_id: &str) -> Result<Option<section::Model>> {
        let section = section::Entity::find()
            .filter(section::Column::RemoteId.eq(remote_id))
            .one(&self.conn)
            .await?;

        Ok(section)
    }

    /// Delete all sections
    pub async fn delete_all_sections(&self) -> Result<()> {
        section::Entity::delete_many().exec(&self.conn).await?;
        Ok(())
    }

    /// Get all sections ordered by order_index
    pub async fn get_all_sections(&self) -> Result<Vec<section::Model>> {
        let sections = section::Entity::find()
            .order_by_asc(section::Column::OrderIndex)
            .all(&self.conn)
            .await?;

        Ok(sections)
    }

    /// Get all sections as SectionDisplay (for UI)
    pub async fn get_sections(&self) -> Result<Vec<SectionDisplay>> {
        let sections = self.get_all_sections().await?;
        Ok(sections.into_iter().map(|s| s.into()).collect())
    }

    /// Get sections for a project as SectionDisplay (for UI)
    pub async fn get_sections_for_project(&self, project_uuid: &str) -> Result<Vec<SectionDisplay>> {
        let sections = self.get_sections_by_project_uuid(project_uuid).await?;
        Ok(sections.into_iter().map(|s| s.into()).collect())
    }
}
