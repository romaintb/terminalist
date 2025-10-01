use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "tasks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub uuid: String,
    #[sea_orm(unique)]
    pub remote_id: String,
    pub content: String,
    pub description: Option<String>,
    pub project_uuid: String,
    pub section_uuid: Option<String>,
    pub parent_uuid: Option<String>,
    pub priority: i32,
    pub order_index: i32,
    pub due_date: Option<String>,
    pub due_datetime: Option<String>,
    pub is_recurring: bool,
    pub deadline: Option<String>,
    pub duration: Option<String>,
    pub is_completed: bool,
    pub is_deleted: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::project::Entity",
        from = "Column::ProjectUuid",
        to = "super::project::Column::Uuid",
        on_delete = "Cascade"
    )]
    Project,
    #[sea_orm(
        belongs_to = "super::section::Entity",
        from = "Column::SectionUuid",
        to = "super::section::Column::Uuid",
        on_delete = "SetNull"
    )]
    Section,
    #[sea_orm(
        belongs_to = "Entity",
        from = "Column::ParentUuid",
        to = "Column::Uuid",
        on_delete = "Cascade"
    )]
    Parent,
}

impl Related<super::project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Project.def()
    }
}

impl Related<super::section::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Section.def()
    }
}

impl Related<super::label::Entity> for Entity {
    fn to() -> RelationDef {
        super::task_label::Relation::Label.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::task_label::Relation::Task.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
