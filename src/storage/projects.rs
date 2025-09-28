use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::Row;

use super::db::LocalStorage;
use crate::todoist::ProjectDisplay;

/// Local project representation with sync metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalProject {
    pub uuid: String,
    pub backend_id: String,
    pub external_id: String,
    pub name: String,
    pub color: String,
    pub is_favorite: bool,
    pub is_inbox_project: bool,
    pub order_index: i32,
    pub parent_uuid: Option<String>,
}

impl From<LocalProject> for ProjectDisplay {
    fn from(local: LocalProject) -> Self {
        Self {
            uuid: local.uuid,
            name: local.name,
            color: local.color,
            is_favorite: local.is_favorite,
            parent_uuid: local.parent_uuid,
            is_inbox_project: local.is_inbox_project,
        }
    }
}

impl LocalStorage {
    /// Store projects from a specific backend
    pub async fn store_projects_for_backend(
        &self,
        backend_id: &str,
        projects: Vec<todoist_api::Project>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Clear existing projects for this backend only
        sqlx::query("DELETE FROM projects WHERE backend_id = ?")
            .bind(backend_id)
            .execute(&mut *tx)
            .await?;

        // First pass: Insert all projects without parent relationships
        for project in &projects {
            // Check if project already exists for this backend
            let existing_uuid: Option<String> = sqlx::query_scalar(
                "SELECT uuid FROM projects WHERE backend_id = ? AND external_id = ?"
            )
            .bind(backend_id)
            .bind(&project.id)
            .fetch_optional(&mut *tx)
            .await?;

            let uuid = existing_uuid.clone().unwrap_or_else(|| Self::generate_uuid());

            if existing_uuid.is_some() {
                // Update existing project (without changing parent_uuid yet)
                sqlx::query(
                    r"
                    UPDATE projects
                    SET name = ?, color = ?, is_favorite = ?, is_inbox_project = ?, order_index = ?, parent_uuid = NULL
                    WHERE uuid = ?
                    "
                )
                .bind(&project.name)
                .bind(&project.color)
                .bind(project.is_favorite)
                .bind(project.is_inbox_project)
                .bind(project.order)
                .bind(&uuid)
                .execute(&mut *tx)
                .await?;
            } else {
                // Insert new project
                sqlx::query(
                    r"
                    INSERT INTO projects (uuid, backend_id, external_id, name, color, is_favorite, is_inbox_project, order_index, parent_uuid)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, NULL)
                    ",
                )
                .bind(&uuid)
                .bind(backend_id)
                .bind(&project.id)
                .bind(&project.name)
                .bind(&project.color)
                .bind(project.is_favorite)
                .bind(project.is_inbox_project)
                .bind(project.order)
                .execute(&mut *tx)
                .await?;
            }
        }

        // Second pass: Update parent relationships now that all projects exist
        for project in &projects {
            if let Some(parent_external_id) = &project.parent_id {
                // Find the UUID of the parent project by its external_id
                let parent_uuid: Option<String> = sqlx::query_scalar(
                    "SELECT uuid FROM projects WHERE backend_id = ? AND external_id = ?"
                )
                .bind(backend_id)
                .bind(parent_external_id)
                .fetch_optional(&mut *tx)
                .await?;

                if let Some(parent_uuid) = parent_uuid {
                    // Update the project with its parent UUID
                    sqlx::query(
                        "UPDATE projects SET parent_uuid = ? WHERE backend_id = ? AND external_id = ?"
                    )
                    .bind(&parent_uuid)
                    .bind(backend_id)
                    .bind(&project.id)
                    .execute(&mut *tx)
                    .await?;
                }
            }
        }

        tx.commit().await?;
        Ok(())
    }

    /// Store projects in local database (legacy method - clears all backends)
    pub async fn store_projects(&self, projects: Vec<todoist_api::Project>) -> Result<()> {
        // For backward compatibility, assume single "todoist" backend
        self.store_projects_for_backend("todoist", projects).await
    }

    /// Store a single project in the database for a specific backend
    pub async fn store_single_project_for_backend(
        &self,
        backend_id: &str,
        project: todoist_api::Project,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Check if project already exists for this backend
        let existing_uuid: Option<String> = sqlx::query_scalar(
            "SELECT uuid FROM projects WHERE backend_id = ? AND external_id = ?"
        )
        .bind(backend_id)
        .bind(&project.id)
        .fetch_optional(&mut *tx)
        .await?;

        let uuid = existing_uuid.clone().unwrap_or_else(|| Self::generate_uuid());

        // Resolve parent UUID if parent_id is provided
        let parent_uuid = if let Some(parent_external_id) = &project.parent_id {
            sqlx::query_scalar::<_, String>(
                "SELECT uuid FROM projects WHERE backend_id = ? AND external_id = ?"
            )
            .bind(backend_id)
            .bind(parent_external_id)
            .fetch_optional(&mut *tx)
            .await?
        } else {
            None
        };

        if existing_uuid.is_some() {
            // Update existing project
            sqlx::query(
                r"
                UPDATE projects
                SET name = ?, color = ?, is_favorite = ?, is_inbox_project = ?, order_index = ?, parent_uuid = ?
                WHERE uuid = ?
                "
            )
            .bind(&project.name)
            .bind(&project.color)
            .bind(project.is_favorite)
            .bind(project.is_inbox_project)
            .bind(project.order)
            .bind(parent_uuid)
            .bind(&uuid)
            .execute(&mut *tx)
            .await?;
        } else {
            // Insert new project
            sqlx::query(
                r"
                INSERT INTO projects (uuid, backend_id, external_id, name, color, is_favorite, is_inbox_project, order_index, parent_uuid)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                ",
            )
            .bind(&uuid)
            .bind(backend_id)
            .bind(&project.id)
            .bind(&project.name)
            .bind(&project.color)
            .bind(project.is_favorite)
            .bind(project.is_inbox_project)
            .bind(project.order)
            .bind(parent_uuid)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Store a single project in the database (legacy method - assumes "todoist" backend)
    pub async fn store_single_project(&self, project: todoist_api::Project) -> Result<()> {
        self.store_single_project_for_backend("todoist", project).await
    }

    /// Delete a project and all its tasks by UUID
    pub async fn delete_project(&self, project_uuid: &str) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Delete tasks first (CASCADE should handle this, but let's be explicit)
        sqlx::query("DELETE FROM tasks WHERE project_uuid = ?")
            .bind(project_uuid)
            .execute(&mut *tx)
            .await?;

        // Delete sections
        sqlx::query("DELETE FROM sections WHERE project_uuid = ?")
            .bind(project_uuid)
            .execute(&mut *tx)
            .await?;

        // Delete the project
        sqlx::query("DELETE FROM projects WHERE uuid = ?")
            .bind(project_uuid)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Update project name by UUID
    pub async fn update_project_name(&self, project_uuid: &str, name: &str) -> Result<()> {
        sqlx::query("UPDATE projects SET name = ? WHERE uuid = ?")
            .bind(name)
            .bind(project_uuid)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get all projects from local storage (all backends)
    pub async fn get_projects(&self) -> Result<Vec<ProjectDisplay>> {
        let rows = sqlx::query(
            r"
            SELECT uuid, name, color, is_favorite, parent_uuid, is_inbox_project
            FROM projects
            ORDER BY order_index, name
            ",
        )
        .fetch_all(&self.pool)
        .await?;

        let projects = rows
            .into_iter()
            .map(|row| ProjectDisplay {
                uuid: row.get("uuid"),
                name: row.get("name"),
                color: row.get("color"),
                is_favorite: row.get("is_favorite"),
                parent_uuid: row.get("parent_uuid"),
                is_inbox_project: row.get("is_inbox_project"),
            })
            .collect();

        Ok(projects)
    }

    /// Get projects for a specific backend
    pub async fn get_projects_for_backend(&self, backend_id: &str) -> Result<Vec<ProjectDisplay>> {
        let rows = sqlx::query(
            r"
            SELECT uuid, name, color, is_favorite, parent_uuid, is_inbox_project
            FROM projects
            WHERE backend_id = ?
            ORDER BY order_index, name
            ",
        )
        .bind(backend_id)
        .fetch_all(&self.pool)
        .await?;

        let projects = rows
            .into_iter()
            .map(|row| ProjectDisplay {
                uuid: row.get("uuid"),
                name: row.get("name"),
                color: row.get("color"),
                is_favorite: row.get("is_favorite"),
                parent_uuid: row.get("parent_uuid"),
                is_inbox_project: row.get("is_inbox_project"),
            })
            .collect();

        Ok(projects)
    }

    /// Sync a project from a backend (maintains existing UUID if found)
    pub async fn sync_project_from_backend(
        &self,
        backend_id: &str,
        project: &todoist_api::Project,
    ) -> Result<()> {
        // Check if we already have this project for this backend
        let existing_uuid = sqlx::query_scalar::<_, String>(
            "SELECT uuid FROM projects WHERE backend_id = ? AND external_id = ?"
        )
        .bind(backend_id)
        .bind(&project.id)
        .fetch_optional(&self.pool)
        .await?;

        let uuid = existing_uuid.unwrap_or_else(|| Self::generate_uuid());

        // Insert or update the project
        sqlx::query(
            r"
            INSERT OR REPLACE INTO projects
            (uuid, backend_id, external_id, name, color, is_favorite, is_inbox_project, order_index, parent_uuid)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, NULL)
            ",
        )
        .bind(&uuid)
        .bind(backend_id)
        .bind(&project.id)
        .bind(&project.name)
        .bind(&project.color)
        .bind(project.is_favorite)
        .bind(project.is_inbox_project)
        .bind(project.order)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
