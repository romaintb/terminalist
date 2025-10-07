//! Backend entity for storing configured backend instances.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "backends")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub uuid: Uuid,
    pub backend_type: String,
    pub name: String,
    pub config: String, // JSON-encoded configuration (tokens, credentials, etc.)
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::project::Entity")]
    Projects,
    #[sea_orm(has_many = "super::task::Entity")]
    Tasks,
    #[sea_orm(has_many = "super::label::Entity")]
    Labels,
    #[sea_orm(has_many = "super::section::Entity")]
    Sections,
}

impl Related<super::project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Projects.def()
    }
}

impl Related<super::task::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tasks.def()
    }
}

impl Related<super::label::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Labels.def()
    }
}

impl Related<super::section::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Sections.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
