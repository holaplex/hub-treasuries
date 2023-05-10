//! `SeaORM` Entity. Generated by sea-orm-codegen 0.11.0

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "customer_treasuries")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub customer_id: Uuid,
    #[sea_orm(unique)]
    pub treasury_id: Uuid,
    pub created_at: DateTimeWithTimeZone,
    pub project_id: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_one = "super::treasuries::Entity")]
    Treasury,
    #[sea_orm(has_many = "super::wallets::Entity")]
    Wallets,
}

impl Related<super::treasuries::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Treasury.def()
    }
}

impl Related<super::wallets::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Wallets.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
