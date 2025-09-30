use anyhow::Result;

use super::db::LocalStorage;
use crate::todoist::{Section, SectionDisplay};

/// Local section representation with sync metadata
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LocalSection {
    pub uuid: String,
    pub remote_id: String,
    pub name: String,
    pub project_uuid: String,
    pub order_index: i32,
}

impl From<LocalSection> for SectionDisplay {
    fn from(local: LocalSection) -> Self {
        Self {
            id: local.uuid,
            name: local.name,
            project_id: local.project_uuid,
            order: local.order_index,
        }
    }
}

impl From<Section> for LocalSection {
    fn from(section: Section) -> Self {
        Self {
            uuid: uuid::Uuid::new_v4().to_string(),
            remote_id: section.id,
            name: section.name,
            project_uuid: String::new(), // Will be resolved at storage layer
            order_index: section.order,
        }
    }
}

impl LocalStorage {
    /// Store sections in local database
    pub async fn store_sections(&self, sections: Vec<Section>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Upsert sections with mapped project_uuid
        // This preserves existing UUIDs when remote_id matches
        for section in &sections {
            let mut local_section: LocalSection = section.clone().into();

            // Look up local project UUID from remote project_id
            if let Some(local_project_uuid) =
                self.find_uuid_by_remote_id(&mut tx, "projects", &section.project_id).await?
            {
                local_section.project_uuid = local_project_uuid;
            }

            sqlx::query(
                r"
                INSERT INTO sections (uuid, remote_id, name, project_uuid, order_index)
                VALUES (?, ?, ?, ?, ?)
                ON CONFLICT(remote_id) DO UPDATE SET
                    name = excluded.name,
                    project_uuid = excluded.project_uuid,
                    order_index = excluded.order_index
                ",
            )
            .bind(&local_section.uuid)
            .bind(&local_section.remote_id)
            .bind(&local_section.name)
            .bind(&local_section.project_uuid)
            .bind(local_section.order_index)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Get all sections from local storage
    pub async fn get_sections(&self) -> Result<Vec<SectionDisplay>> {
        let sections = sqlx::query_as::<_, LocalSection>(
            "SELECT uuid, remote_id, name, project_uuid, order_index FROM sections ORDER BY project_uuid, order_index, name",
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|local| local.into())
        .collect();

        Ok(sections)
    }

    /// Get sections for a specific project from local storage
    pub async fn get_sections_for_project(&self, project_id: &str) -> Result<Vec<SectionDisplay>> {
        let sections = sqlx::query_as::<_, LocalSection>(
            "SELECT uuid, remote_id, name, project_uuid, order_index FROM sections WHERE project_uuid = ? ORDER BY order_index, name",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|local| local.into())
        .collect();

        Ok(sections)
    }
}
