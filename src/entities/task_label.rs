use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "task_labels")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub task_uuid: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub label_uuid: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::task::Entity",
        from = "Column::TaskUuid",
        to = "super::task::Column::Uuid",
        on_delete = "Cascade"
    )]
    Task,
    #[sea_orm(
        belongs_to = "super::label::Entity",
        from = "Column::LabelUuid",
        to = "super::label::Column::Uuid",
        on_delete = "Cascade"
    )]
    Label,
}

impl ActiveModelBehavior for ActiveModel {}
