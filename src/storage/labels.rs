use anyhow::Result;
use sea_orm::{prelude::*, ActiveValue, QueryOrder};

use crate::entities::label;
use crate::todoist::Label as TodoistLabel;

use super::LocalStorage;

impl LocalStorage {
    /// Store a label in the database
    pub async fn store_label(&self, label_data: &TodoistLabel) -> Result<()> {
        let model = label::ActiveModel {
            uuid: ActiveValue::Set(Uuid::new_v4().to_string()),
            remote_id: ActiveValue::Set(label_data.id.clone()),
            name: ActiveValue::Set(label_data.name.clone()),
            color: ActiveValue::Set(label_data.color.clone()),
            order_index: ActiveValue::Set(label_data.order),
            is_favorite: ActiveValue::Set(label_data.is_favorite),
        };

        label::Entity::insert(model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(label::Column::RemoteId)
                    .update_columns([
                        label::Column::Name,
                        label::Column::Color,
                        label::Column::OrderIndex,
                        label::Column::IsFavorite,
                    ])
                    .to_owned(),
            )
            .exec(&self.conn)
            .await?;

        Ok(())
    }

    /// Get all labels ordered by order_index
    pub async fn get_all_labels(&self) -> Result<Vec<label::Model>> {
        let labels = label::Entity::find()
            .order_by_asc(label::Column::OrderIndex)
            .all(&self.conn)
            .await?;

        Ok(labels)
    }

    /// Get a label by its remote ID
    pub async fn get_label_by_remote_id(&self, remote_id: &str) -> Result<Option<label::Model>> {
        let label = label::Entity::find()
            .filter(label::Column::RemoteId.eq(remote_id))
            .one(&self.conn)
            .await?;

        Ok(label)
    }

    /// Delete all labels
    pub async fn delete_all_labels(&self) -> Result<()> {
        label::Entity::delete_many().exec(&self.conn).await?;
        Ok(())
    }
}
