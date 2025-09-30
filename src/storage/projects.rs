use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::db::LocalStorage;
use crate::todoist::{Project, ProjectDisplay};

/// Local project representation with sync metadata
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LocalProject {
    pub uuid: String,
    pub remote_id: String,
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
            id: local.uuid,
            name: local.name,
            color: local.color,
            is_favorite: local.is_favorite,
            parent_id: local.parent_uuid,
            is_inbox_project: local.is_inbox_project,
        }
    }
}

impl From<Project> for LocalProject {
    fn from(project: Project) -> Self {
        Self {
            uuid: uuid::Uuid::new_v4().to_string(),
            remote_id: project.id,
            name: project.name,
            color: project.color,
            is_favorite: project.is_favorite,
            is_inbox_project: project.is_inbox_project,
            order_index: project.order,
            parent_uuid: None, // Will be resolved at storage layer
        }
    }
}

impl LocalStorage {
    /// Store projects in local database
    pub async fn store_projects(&self, projects: Vec<Project>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // First pass: Upsert all projects without parent_uuid relationships
        // This preserves existing UUIDs when remote_id matches
        for project in &projects {
            let local_project: LocalProject = project.clone().into();

            sqlx::query(
                r"
                INSERT INTO projects (uuid, remote_id, name, color, is_favorite, is_inbox_project, order_index, parent_uuid)
                VALUES (?, ?, ?, ?, ?, ?, ?, NULL)
                ON CONFLICT(remote_id) DO UPDATE SET
                    name = excluded.name,
                    color = excluded.color,
                    is_favorite = excluded.is_favorite,
                    is_inbox_project = excluded.is_inbox_project,
                    order_index = excluded.order_index,
                    parent_uuid = NULL
                ",
            )
            .bind(&local_project.uuid)
            .bind(&local_project.remote_id)
            .bind(&local_project.name)
            .bind(&local_project.color)
            .bind(local_project.is_favorite)
            .bind(local_project.is_inbox_project)
            .bind(local_project.order_index)
            .execute(&mut *tx)
            .await?;
        }

        // Second pass: Update parent_uuid references to use local UUIDs
        for project in &projects {
            if let Some(remote_parent_id) = &project.parent_id {
                if let Some(local_parent_uuid) =
                    self.find_uuid_by_remote_id(&mut tx, "projects", remote_parent_id).await?
                {
                    sqlx::query("UPDATE projects SET parent_uuid = ? WHERE remote_id = ?")
                        .bind(&local_parent_uuid)
                        .bind(&project.id)
                        .execute(&mut *tx)
                        .await?;
                }
            }
        }

        tx.commit().await?;
        Ok(())
    }

    /// Store a single project in the database (for immediate insertion after API calls)
    pub async fn store_single_project(&self, project: Project) -> Result<()> {
        let local_project: LocalProject = project.into();

        sqlx::query(
            r"
            INSERT OR REPLACE INTO projects (uuid, remote_id, name, color, is_favorite, is_inbox_project, order_index, parent_uuid)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(&local_project.uuid)
        .bind(&local_project.remote_id)
        .bind(&local_project.name)
        .bind(&local_project.color)
        .bind(local_project.is_favorite)
        .bind(local_project.is_inbox_project)
        .bind(local_project.order_index)
        .bind(&local_project.parent_uuid)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete a project and all its tasks
    pub async fn delete_project(&self, project_id: &str) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Delete tasks first, then the project
        sqlx::query("DELETE FROM tasks WHERE project_uuid = ?")
            .bind(project_id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM projects WHERE uuid = ?")
            .bind(project_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Update project name in local storage
    pub async fn update_project_name(&self, project_id: &str, name: &str) -> Result<()> {
        sqlx::query("UPDATE projects SET name = ? WHERE uuid = ?")
            .bind(name)
            .bind(project_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get all projects from local storage
    pub async fn get_projects(&self) -> Result<Vec<ProjectDisplay>> {
        let projects = sqlx::query_as::<_, LocalProject>(
            "SELECT uuid, remote_id, name, color, is_favorite, parent_uuid, is_inbox_project, order_index FROM projects ORDER BY order_index, name"
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|local| local.into())
        .collect();

        Ok(projects)
    }
}
