use sea_orm::entity::prelude::*;
use sea_orm::QueryOrder;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "tasks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub uuid: Uuid,
    pub backend_uuid: Uuid,
    pub remote_id: String,
    pub content: String,
    pub description: Option<String>,
    pub project_uuid: Uuid,
    pub section_uuid: Option<Uuid>,
    pub parent_uuid: Option<Uuid>,
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
    #[sea_orm(
        belongs_to = "super::backend::Entity",
        from = "Column::BackendUuid",
        to = "super::backend::Column::Uuid",
        on_delete = "Cascade"
    )]
    Backend,
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

impl Related<super::backend::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Backend.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Entity {
    /// Scope for overdue tasks (due before today)
    /// Orders by: deleted status, completion status, due date
    pub fn overdue(today: &str) -> Select<Entity> {
        Self::find()
            .filter(Column::DueDate.is_not_null())
            .filter(Column::DueDate.lt(today))
            .order_by_asc(Column::IsDeleted)
            .order_by_asc(Column::IsCompleted)
            .order_by_asc(Column::DueDate)
    }

    /// Scope for tasks due today
    /// Orders by: deleted status, completion status, order index
    pub fn due_today(today: &str) -> Select<Entity> {
        Self::find()
            .filter(Column::DueDate.eq(today))
            .order_by_asc(Column::IsDeleted)
            .order_by_asc(Column::IsCompleted)
            .order_by_asc(Column::OrderIndex)
    }

    /// Scope for tasks due in a date range
    /// Orders by: deleted status, completion status, due date
    pub fn due_between(start: &str, end: &str) -> Select<Entity> {
        Self::find()
            .filter(Column::DueDate.gte(start))
            .filter(Column::DueDate.lt(end))
            .order_by_asc(Column::IsDeleted)
            .order_by_asc(Column::IsCompleted)
            .order_by_asc(Column::DueDate)
    }
}
