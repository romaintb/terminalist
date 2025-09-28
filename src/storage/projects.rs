use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::db::LocalStorage;
use crate::todoist::{Project, ProjectDisplay};

/// Local project representation with sync metadata
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LocalProject {
    pub id: String,
    pub name: String,
    pub color: String,
    pub is_favorite: bool,
    pub is_inbox_project: bool,
    pub order_index: i32,
    pub parent_id: Option<String>,
}

impl From<LocalProject> for ProjectDisplay {
    fn from(local: LocalProject) -> Self {
        Self {
            id: local.id,
            name: local.name,
            color: local.color,
            is_favorite: local.is_favorite,
            parent_id: local.parent_id,
            is_inbox_project: local.is_inbox_project,
        }
    }
}

impl From<Project> for LocalProject {
    fn from(project: Project) -> Self {
        Self {
            id: project.id,
            name: project.name,
            color: project.color,
            is_favorite: project.is_favorite,
            is_inbox_project: project.is_inbox_project,
            order_index: project.order,
            parent_id: project.parent_id,
        }
    }
}

impl LocalStorage {
    /// Store projects in local database
    pub async fn store_projects(&self, projects: Vec<Project>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Clear existing projects
        sqlx::query("DELETE FROM projects").execute(&mut *tx).await?;

        // Insert new projects
        for project in &projects {
            let local_project: LocalProject = project.clone().into();
            sqlx::query(
                r"
                INSERT INTO projects (id, name, color, is_favorite, is_inbox_project, order_index, parent_id)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                ",
            )
            .bind(&local_project.id)
            .bind(&local_project.name)
            .bind(&local_project.color)
            .bind(local_project.is_favorite)
            .bind(local_project.is_inbox_project)
            .bind(local_project.order_index)
            .bind(&local_project.parent_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Store a single project in the database (for immediate insertion after API calls)
    pub async fn store_single_project(&self, project: Project) -> Result<()> {
        let local_project: LocalProject = project.into();

        sqlx::query(
            r"
            INSERT OR REPLACE INTO projects (id, name, color, is_favorite, is_inbox_project, order_index, parent_id)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(&local_project.id)
        .bind(&local_project.name)
        .bind(&local_project.color)
        .bind(local_project.is_favorite)
        .bind(local_project.is_inbox_project)
        .bind(local_project.order_index)
        .bind(&local_project.parent_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete a project and all its tasks
    pub async fn delete_project(&self, project_id: &str) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Delete tasks first, then the project
        sqlx::query("DELETE FROM tasks WHERE project_id = ?")
            .bind(project_id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(project_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Update project name in local storage
    pub async fn update_project_name(&self, project_id: &str, name: &str) -> Result<()> {
        sqlx::query("UPDATE projects SET name = ? WHERE id = ?")
            .bind(name)
            .bind(project_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get all projects from local storage
    pub async fn get_projects(&self) -> Result<Vec<ProjectDisplay>> {
        let projects = sqlx::query_as::<_, LocalProject>(
            "SELECT id, name, color, is_favorite, parent_id, is_inbox_project, order_index FROM projects ORDER BY order_index, name"
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|local| local.into())
        .collect();

        Ok(projects)
    }
}
