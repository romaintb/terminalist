use anyhow::Result;
use sqlx::Row;

use super::db::LocalStorage;
use crate::todoist::{Section, SectionDisplay};

/// Local section representation with sync metadata
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LocalSection {
    pub uuid: String,
    pub external_id: String,
    pub name: String,
    pub project_uuid: String,
    pub order_index: i32,
}

impl From<LocalSection> for SectionDisplay {
    fn from(local: LocalSection) -> Self {
        Self {
            uuid: local.uuid,
            name: local.name,
            project_id: local.project_uuid,
            order: local.order_index,
        }
    }
}

impl LocalSection {
    /// Convert from API Section to LocalSection with context
    pub fn from_section_with_context(section: Section, project_uuid: String) -> Self {
        Self {
            uuid: LocalStorage::generate_uuid(),
            external_id: section.id,
            name: section.name,
            project_uuid,
            order_index: section.order,
        }
    }
}

impl LocalStorage {
    /// Store sections for a specific backend
    pub async fn store_sections_for_backend(
        &self,
        backend_id: &str,
        sections: Vec<Section>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Clear existing sections for this backend (through project relationship)
        sqlx::query(
            r"
            DELETE FROM sections
            WHERE project_uuid IN (
                SELECT uuid FROM projects WHERE backend_id = ?
            )
            ",
        )
        .bind(backend_id)
        .execute(&mut *tx)
        .await?;

        // Insert new sections
        // For now, we'll skip project relationships during sync since we need to resolve UUIDs
        // The UI will still work fine as it uses UUIDs for everything
        for section in &sections {
            let local_section = LocalSection::from_section_with_context(section.clone(), "".to_string()); // Empty project UUID for now
            sqlx::query(
                r"
                INSERT OR REPLACE INTO sections (uuid, external_id, name, project_uuid, order_index)
                VALUES (?, ?, ?, NULL, ?)
                ",
            )
            .bind(&local_section.uuid)
            .bind(&local_section.external_id)
            .bind(&local_section.name)
            .bind(local_section.order_index)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Store sections in local database (legacy method - clears all backends)
    pub async fn store_sections(&self, sections: Vec<Section>) -> Result<()> {
        // For backward compatibility, assume single "todoist" backend
        self.store_sections_for_backend("todoist", sections).await
    }

    /// Get all sections from local storage
    pub async fn get_sections(&self) -> Result<Vec<SectionDisplay>> {
        let rows = sqlx::query(
            r"
            SELECT uuid, name, project_uuid, order_index
            FROM sections
            ORDER BY project_uuid, order_index, name
            ",
        )
        .fetch_all(&self.pool)
        .await?;

        let sections = rows
            .into_iter()
            .map(|row| SectionDisplay {
                uuid: row.get("uuid"),
                name: row.get("name"),
                project_id: row.get("project_uuid"),
                order: row.get("order_index"),
            })
            .collect();

        Ok(sections)
    }

    /// Get sections for a specific project from local storage
    pub async fn get_sections_for_project(&self, project_uuid: &str) -> Result<Vec<SectionDisplay>> {
        let rows = sqlx::query(
            r"
            SELECT uuid, name, project_uuid, order_index
            FROM sections
            WHERE project_uuid = ?
            ORDER BY order_index, name
            ",
        )
        .bind(project_uuid)
        .fetch_all(&self.pool)
        .await?;

        let sections = rows
            .into_iter()
            .map(|row| SectionDisplay {
                uuid: row.get("uuid"),
                name: row.get("name"),
                project_id: row.get("project_uuid"),
                order: row.get("order_index"),
            })
            .collect();

        Ok(sections)
    }

    /// Get sections for a specific backend
    pub async fn get_sections_for_backend(&self, backend_id: &str) -> Result<Vec<SectionDisplay>> {
        let rows = sqlx::query(
            r"
            SELECT s.uuid, s.name, s.project_uuid, s.order_index
            FROM sections s
            JOIN projects p ON s.project_uuid = p.uuid
            WHERE p.backend_id = ?
            ORDER BY s.project_uuid, s.order_index, s.name
            ",
        )
        .bind(backend_id)
        .fetch_all(&self.pool)
        .await?;

        let sections = rows
            .into_iter()
            .map(|row| SectionDisplay {
                uuid: row.get("uuid"),
                name: row.get("name"),
                project_id: row.get("project_uuid"),
                order: row.get("order_index"),
            })
            .collect();

        Ok(sections)
    }

    /// Sync a section from a backend (maintains existing UUID if found)
    pub async fn sync_section_from_backend(
        &self,
        backend_id: &str,
        section: &todoist_api::Section,
    ) -> Result<()> {
        // Check if we already have this section for this backend
        let existing_uuid = sqlx::query_scalar::<_, String>(
            "SELECT uuid FROM sections WHERE external_id = ?"
        )
        .bind(&section.id)
        .fetch_optional(&self.pool)
        .await?;

        let uuid = existing_uuid.unwrap_or_else(|| Self::generate_uuid());

        // Find the project UUID for this section by matching project external_id
        let project_uuid = sqlx::query_scalar::<_, String>(
            "SELECT uuid FROM projects WHERE backend_id = ? AND external_id = ?"
        )
        .bind(backend_id)
        .bind(&section.project_id)
        .fetch_optional(&self.pool)
        .await?;

        // Insert or update the section
        sqlx::query(
            r"
            INSERT OR REPLACE INTO sections
            (uuid, external_id, name, project_uuid, order_index)
            VALUES (?, ?, ?, ?, ?)
            ",
        )
        .bind(&uuid)
        .bind(&section.id)
        .bind(&section.name)
        .bind(project_uuid) // May be None if project not found yet
        .bind(section.order)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
