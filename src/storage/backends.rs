use anyhow::Result;
use sqlx::Row;

use super::db::LocalStorage;

/// Represents a registered backend in the database
#[derive(Debug, Clone)]
pub struct RegisteredBackend {
    pub backend_id: String,
    pub backend_type: String,
    pub name: String,
    pub enabled: bool,
    pub last_sync: Option<String>,
    pub sync_error: Option<String>,
    pub config: Option<String>,
}

impl LocalStorage {
    /// Register a backend instance in the database
    pub async fn register_backend(
        &self,
        backend_id: &str,
        backend_type: &str,
        name: &str,
        enabled: bool,
        config: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r"
            INSERT OR REPLACE INTO backends (backend_id, backend_type, name, enabled, config)
            VALUES (?, ?, ?, ?, ?)
            ",
        )
        .bind(backend_id)
        .bind(backend_type)
        .bind(name)
        .bind(enabled)
        .bind(config)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update backend sync status
    pub async fn update_backend_sync_status(
        &self,
        backend_id: &str,
        last_sync: Option<&str>,
        sync_error: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r"
            UPDATE backends
            SET last_sync = ?, sync_error = ?
            WHERE backend_id = ?
            ",
        )
        .bind(last_sync)
        .bind(sync_error)
        .bind(backend_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all registered backends
    pub async fn get_registered_backends(&self) -> Result<Vec<RegisteredBackend>> {
        let rows = sqlx::query(
            r"
            SELECT backend_id, backend_type, name, enabled, last_sync, sync_error, config
            FROM backends
            ORDER BY backend_id
            ",
        )
        .fetch_all(&self.pool)
        .await?;

        let backends = rows
            .into_iter()
            .map(|row| RegisteredBackend {
                backend_id: row.get("backend_id"),
                backend_type: row.get("backend_type"),
                name: row.get("name"),
                enabled: row.get("enabled"),
                last_sync: row.get("last_sync"),
                sync_error: row.get("sync_error"),
                config: row.get("config"),
            })
            .collect();

        Ok(backends)
    }

    /// Get a specific registered backend by ID
    pub async fn get_registered_backend(&self, backend_id: &str) -> Result<Option<RegisteredBackend>> {
        let row = sqlx::query(
            r"
            SELECT backend_id, backend_type, name, enabled, last_sync, sync_error, config
            FROM backends
            WHERE backend_id = ?
            ",
        )
        .bind(backend_id)
        .fetch_optional(&self.pool)
        .await?;

        let backend = row.map(|row| RegisteredBackend {
            backend_id: row.get("backend_id"),
            backend_type: row.get("backend_type"),
            name: row.get("name"),
            enabled: row.get("enabled"),
            last_sync: row.get("last_sync"),
            sync_error: row.get("sync_error"),
            config: row.get("config"),
        });

        Ok(backend)
    }

    /// Enable or disable a backend
    pub async fn set_backend_enabled(&self, backend_id: &str, enabled: bool) -> Result<()> {
        sqlx::query("UPDATE backends SET enabled = ? WHERE backend_id = ?")
            .bind(enabled)
            .bind(backend_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Remove a backend from the registry
    pub async fn remove_backend(&self, backend_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM backends WHERE backend_id = ?")
            .bind(backend_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get backends by type
    pub async fn get_backends_by_type(&self, backend_type: &str) -> Result<Vec<RegisteredBackend>> {
        let rows = sqlx::query(
            r"
            SELECT backend_id, backend_type, name, enabled, last_sync, sync_error, config
            FROM backends
            WHERE backend_type = ?
            ORDER BY backend_id
            ",
        )
        .bind(backend_type)
        .fetch_all(&self.pool)
        .await?;

        let backends = rows
            .into_iter()
            .map(|row| RegisteredBackend {
                backend_id: row.get("backend_id"),
                backend_type: row.get("backend_type"),
                name: row.get("name"),
                enabled: row.get("enabled"),
                last_sync: row.get("last_sync"),
                sync_error: row.get("sync_error"),
                config: row.get("config"),
            })
            .collect();

        Ok(backends)
    }

    /// Clear sync error for a backend
    pub async fn clear_backend_sync_error(&self, backend_id: &str) -> Result<()> {
        sqlx::query("UPDATE backends SET sync_error = NULL WHERE backend_id = ?")
            .bind(backend_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}