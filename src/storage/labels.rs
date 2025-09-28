use anyhow::Result;
use sqlx::Row;

use super::db::LocalStorage;
use crate::todoist::Label;

/// Local label representation with sync metadata
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LocalLabel {
    pub uuid: String,
    pub backend_id: String,
    pub external_id: String,
    pub name: String,
    pub color: String,
    pub order_index: i32,
    pub is_favorite: bool,
}

/// Color information for LocalLabel
pub struct LocalLabelColor {
    pub color: String,
}

impl LocalLabel {
    /// Convert from API Label to LocalLabel with backend context
    pub fn from_label_with_backend(label: Label, backend_id: String) -> Self {
        Self {
            uuid: LocalStorage::generate_uuid(),
            backend_id,
            external_id: label.id,
            name: label.name,
            color: label.color,
            order_index: label.order,
            is_favorite: label.is_favorite,
        }
    }
}

impl LocalStorage {
    /// Store a single label for a specific backend
    pub async fn store_single_label_for_backend(
        &self,
        backend_id: &str,
        label: Label,
    ) -> Result<()> {
        // Use INSERT OR REPLACE to handle both new and existing labels
        let local_label = LocalLabel::from_label_with_backend(label, backend_id.to_string());

        sqlx::query(
            r"
            INSERT OR REPLACE INTO labels (uuid, backend_id, external_id, name, color, order_index, is_favorite)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(&local_label.uuid)
        .bind(&local_label.backend_id)
        .bind(&local_label.external_id)
        .bind(&local_label.name)
        .bind(&local_label.color)
        .bind(local_label.order_index)
        .bind(local_label.is_favorite)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Store a single label in the database (legacy method - assumes "todoist" backend)
    pub async fn store_single_label(&self, label: Label) -> Result<()> {
        self.store_single_label_for_backend("todoist", label).await
    }

    /// Store labels for a specific backend
    pub async fn store_labels_for_backend(
        &self,
        backend_id: &str,
        labels: Vec<Label>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Clear existing labels for this backend only
        sqlx::query("DELETE FROM labels WHERE backend_id = ?")
            .bind(backend_id)
            .execute(&mut *tx)
            .await?;

        // Insert new labels
        for label in labels {
            let local_label = LocalLabel::from_label_with_backend(label, backend_id.to_string());
            sqlx::query(
                r"
                INSERT INTO labels (uuid, backend_id, external_id, name, color, order_index, is_favorite)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                ",
            )
            .bind(&local_label.uuid)
            .bind(&local_label.backend_id)
            .bind(&local_label.external_id)
            .bind(&local_label.name)
            .bind(&local_label.color)
            .bind(local_label.order_index)
            .bind(local_label.is_favorite)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Store labels in local database (legacy method - clears all backends)
    pub async fn store_labels(&self, labels: Vec<Label>) -> Result<()> {
        // For backward compatibility, assume single "todoist" backend
        self.store_labels_for_backend("todoist", labels).await
    }

    /// Get all labels from local storage
    pub async fn get_all_labels(&self) -> Result<Vec<LocalLabel>> {
        let rows = sqlx::query(
            r"
            SELECT uuid, backend_id, external_id, name, color, order_index, is_favorite
            FROM labels
            ORDER BY order_index ASC, name ASC
            ",
        )
        .fetch_all(&self.pool)
        .await?;

        let labels = rows
            .into_iter()
            .map(|row| LocalLabel {
                uuid: row.get("uuid"),
                backend_id: row.get("backend_id"),
                external_id: row.get("external_id"),
                name: row.get("name"),
                color: row.get("color"),
                order_index: row.get("order_index"),
                is_favorite: row.get("is_favorite"),
            })
            .collect();

        Ok(labels)
    }

    /// Get labels for a specific backend
    pub async fn get_labels_for_backend(&self, backend_id: &str) -> Result<Vec<LocalLabel>> {
        let rows = sqlx::query(
            r"
            SELECT uuid, backend_id, external_id, name, color, order_index, is_favorite
            FROM labels
            WHERE backend_id = ?
            ORDER BY order_index ASC, name ASC
            ",
        )
        .bind(backend_id)
        .fetch_all(&self.pool)
        .await?;

        let labels = rows
            .into_iter()
            .map(|row| LocalLabel {
                uuid: row.get("uuid"),
                backend_id: row.get("backend_id"),
                external_id: row.get("external_id"),
                name: row.get("name"),
                color: row.get("color"),
                order_index: row.get("order_index"),
                is_favorite: row.get("is_favorite"),
            })
            .collect();

        Ok(labels)
    }

    /// Update label name in local storage
    pub async fn update_label_name(&self, label_uuid: &str, name: &str) -> Result<()> {
        sqlx::query("UPDATE labels SET name = ? WHERE uuid = ?")
            .bind(name)
            .bind(label_uuid)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Sync a label from a backend (maintains existing UUID if found)
    pub async fn sync_label_from_backend(
        &self,
        backend_id: &str,
        label: &todoist_api::Label,
    ) -> Result<()> {
        // Check if we already have this label for this backend
        let existing_uuid = sqlx::query_scalar::<_, String>(
            "SELECT uuid FROM labels WHERE backend_id = ? AND external_id = ?"
        )
        .bind(backend_id)
        .bind(&label.id)
        .fetch_optional(&self.pool)
        .await?;

        let uuid = existing_uuid.unwrap_or_else(|| Self::generate_uuid());

        // Insert or update the label
        sqlx::query(
            r"
            INSERT OR REPLACE INTO labels
            (uuid, backend_id, external_id, name, color, order_index, is_favorite)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(&uuid)
        .bind(backend_id)
        .bind(&label.id)
        .bind(&label.name)
        .bind(&label.color)
        .bind(label.order)
        .bind(label.is_favorite)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
