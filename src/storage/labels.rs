use anyhow::Result;

use super::db::LocalStorage;
use crate::todoist::Label;

/// Local label representation with sync metadata
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LocalLabel {
    pub uuid: String,
    pub remote_id: String,
    pub name: String,
    pub color: String,
    pub order_index: i32,
    pub is_favorite: bool,
}

/// Color information for LocalLabel
pub struct LocalLabelColor {
    pub color: String,
}

impl From<Label> for LocalLabel {
    fn from(label: Label) -> Self {
        Self {
            uuid: uuid::Uuid::new_v4().to_string(),
            remote_id: label.id,
            name: label.name,
            color: label.color,
            order_index: label.order,
            is_favorite: label.is_favorite,
        }
    }
}

impl LocalStorage {
    /// Store a single label in the database (for immediate insertion after API calls)
    pub async fn store_single_label(&self, label: Label) -> Result<()> {
        let local_label: LocalLabel = label.into();

        sqlx::query(
            r"
            INSERT OR REPLACE INTO labels (uuid, remote_id, name, color, order_index, is_favorite)
            VALUES (?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(&local_label.uuid)
        .bind(&local_label.remote_id)
        .bind(&local_label.name)
        .bind(&local_label.color)
        .bind(local_label.order_index)
        .bind(local_label.is_favorite)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Store labels in local database
    pub async fn store_labels(&self, labels: Vec<Label>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Upsert labels to preserve existing UUIDs when remote_id matches
        for label in labels {
            let local_label: LocalLabel = label.into();
            sqlx::query(
                r"
                INSERT INTO labels (uuid, remote_id, name, color, order_index, is_favorite)
                VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT(remote_id) DO UPDATE SET
                    name = excluded.name,
                    color = excluded.color,
                    order_index = excluded.order_index,
                    is_favorite = excluded.is_favorite
                ",
            )
            .bind(&local_label.uuid)
            .bind(&local_label.remote_id)
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

    /// Get all labels from local storage
    pub async fn get_all_labels(&self) -> Result<Vec<LocalLabel>> {
        let labels = sqlx::query_as::<_, LocalLabel>(
            "SELECT uuid, remote_id, name, color, order_index, is_favorite FROM labels ORDER BY order_index ASC, name ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(labels)
    }

    /// Update label name in local storage
    pub async fn update_label_name(&self, label_id: &str, name: &str) -> Result<()> {
        sqlx::query("UPDATE labels SET name = ? WHERE uuid = ?")
            .bind(name)
            .bind(label_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
