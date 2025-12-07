use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "labels")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub uuid: Uuid,
    pub backend_uuid: Uuid,
    pub remote_id: String,
    pub name: String,
    pub order_index: i32,
    pub is_favorite: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::backend::Entity",
        from = "Column::BackendUuid",
        to = "super::backend::Column::Uuid",
        on_delete = "Cascade"
    )]
    Backend,
}

impl Related<super::task::Entity> for Entity {
    fn to() -> RelationDef {
        super::task_label::Relation::Task.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::task_label::Relation::Label.def().rev())
    }
}

impl Related<super::backend::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Backend.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
