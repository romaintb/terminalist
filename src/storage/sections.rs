use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::Row;

use super::db::LocalStorage;
use crate::todoist::{Section, SectionDisplay};

/// Local section representation with sync metadata
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LocalSection {
    pub id: String,
    pub name: String,
    pub project_id: String,
    pub order_index: i32,
    pub last_synced: DateTime<Utc>,
}

impl From<LocalSection> for SectionDisplay {
    fn from(local: LocalSection) -> Self {
        Self {
            id: local.id,
            name: local.name,
            project_id: local.project_id,
            order: local.order_index,
        }
    }
}

impl From<Section> for LocalSection {
    fn from(section: Section) -> Self {
        Self {
            id: section.id,
            name: section.name,
            project_id: section.project_id,
            order_index: section.order,
            last_synced: Utc::now(),
        }
    }
}

impl LocalStorage {
    /// Store sections in local database
    pub async fn store_sections(&self, sections: Vec<Section>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Clear existing sections
        sqlx::query("DELETE FROM sections")
            .execute(&mut *tx)
            .await?;

        // Insert new sections
        for section in &sections {
            let local_section: LocalSection = section.clone().into();
            sqlx::query(
                r"
                INSERT INTO sections (id, name, project_id, order_index, last_synced)
                VALUES (?, ?, ?, ?, ?)
                ",
            )
            .bind(&local_section.id)
            .bind(&local_section.name)
            .bind(&local_section.project_id)
            .bind(local_section.order_index)
            .bind(local_section.last_synced)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.update_sync_timestamp("sections").await?;
        Ok(())
    }

    /// Get all sections from local storage
    pub async fn get_sections(&self) -> Result<Vec<SectionDisplay>> {
        let rows = sqlx::query(
            r"
            SELECT id, name, project_id, order_index
            FROM sections
            ORDER BY project_id, order_index, name
            ",
        )
        .fetch_all(&self.pool)
        .await?;

        let sections = rows
            .into_iter()
            .map(|row| SectionDisplay {
                id: row.get("id"),
                name: row.get("name"),
                project_id: row.get("project_id"),
                order: row.get("order_index"),
            })
            .collect();

        Ok(sections)
    }

    /// Get sections for a specific project from local storage
    pub async fn get_sections_for_project(&self, project_id: &str) -> Result<Vec<SectionDisplay>> {
        let rows = sqlx::query(
            r"
            SELECT id, name, project_id, order_index
            FROM sections
            WHERE project_id = ?
            ORDER BY order_index, name
            ",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;

        let sections = rows
            .into_iter()
            .map(|row| SectionDisplay {
                id: row.get("id"),
                name: row.get("name"),
                project_id: row.get("project_id"),
                order: row.get("order_index"),
            })
            .collect();

        Ok(sections)
    }
}
