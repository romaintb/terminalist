use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::Row;

use super::db::LocalStorage;
use crate::todoist::Label;

/// Local label representation with sync metadata
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LocalLabel {
    pub id: String,
    pub name: String,
    pub color: String,
    pub order_index: i32,
    pub is_favorite: bool,
    pub last_synced: DateTime<Utc>,
}

/// Color information for LocalLabel
pub struct LocalLabelColor {
    pub color: String,
}

impl From<Label> for LocalLabel {
    fn from(label: Label) -> Self {
        Self {
            id: label.id,
            name: label.name,
            color: label.color,
            order_index: label.order,
            is_favorite: label.is_favorite,
            last_synced: Utc::now(),
        }
    }
}

impl LocalStorage {
    /// Store a single label in the database (for immediate insertion after API calls)
    pub async fn store_single_label(&self, label: Label) -> Result<()> {
        let local_label: LocalLabel = label.into();

        sqlx::query(
            r"
            INSERT OR REPLACE INTO labels (id, name, color, order_index, is_favorite, last_synced)
            VALUES (?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(&local_label.id)
        .bind(&local_label.name)
        .bind(&local_label.color)
        .bind(local_label.order_index)
        .bind(local_label.is_favorite)
        .bind(local_label.last_synced)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Store labels in local database
    pub async fn store_labels(&self, labels: Vec<Label>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Clear existing labels
        sqlx::query("DELETE FROM labels").execute(&mut *tx).await?;

        // Insert new labels
        for label in labels {
            let local_label: LocalLabel = label.into();
            sqlx::query(
                r"
                INSERT INTO labels (id, name, color, order_index, is_favorite, last_synced)
                VALUES (?, ?, ?, ?, ?, ?)
                ",
            )
            .bind(&local_label.id)
            .bind(&local_label.name)
            .bind(&local_label.color)
            .bind(local_label.order_index)
            .bind(local_label.is_favorite)
            .bind(local_label.last_synced)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.update_sync_timestamp("labels").await?;
        Ok(())
    }

    /// Get all labels from local storage
    pub async fn get_all_labels(&self) -> Result<Vec<LocalLabel>> {
        let rows = sqlx::query(
            r"
            SELECT id, name, color, order_index, is_favorite, last_synced
            FROM labels
            ORDER BY order_index ASC, name ASC
            ",
        )
        .fetch_all(&self.pool)
        .await?;

        let labels = rows
            .into_iter()
            .map(|row| LocalLabel {
                id: row.get("id"),
                name: row.get("name"),
                color: row.get("color"),
                order_index: row.get("order_index"),
                is_favorite: row.get("is_favorite"),
                last_synced: row.get("last_synced"),
            })
            .collect();

        Ok(labels)
    }

    /// Update label name in local storage
    pub async fn update_label_name(&self, label_id: &str, name: &str) -> Result<()> {
        sqlx::query("UPDATE labels SET name = ?, last_synced = ? WHERE id = ?")
            .bind(name)
            .bind(Utc::now())
            .bind(label_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
