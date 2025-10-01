use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "projects")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub uuid: String,
    #[sea_orm(unique)]
    pub remote_id: String,
    pub name: String,
    pub color: String,
    pub is_favorite: bool,
    pub is_inbox_project: bool,
    pub order_index: i32,
    pub parent_uuid: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::task::Entity")]
    Tasks,
    #[sea_orm(has_many = "super::section::Entity")]
    Sections,
    #[sea_orm(
        belongs_to = "Entity",
        from = "Column::ParentUuid",
        to = "Column::Uuid"
    )]
    Parent,
}

impl Related<super::task::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tasks.def()
    }
}

impl Related<super::section::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Sections.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
