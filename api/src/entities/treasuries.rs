use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize, poem_openapi::Object,
)]
#[sea_orm(table_name = "treasuries")]
#[oai(rename = "Treasury")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub vault_id: String,
    pub created_at: DateTime,
    pub organization_id: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_one = "super::project_treasuries::Entity")]
    ProjectTreasuries,
    #[sea_orm(has_many = "super::wallets::Entity")]
    Wallets,
}

impl Related<super::project_treasuries::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProjectTreasuries.def()
    }
}

impl Related<super::wallets::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Wallets.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
