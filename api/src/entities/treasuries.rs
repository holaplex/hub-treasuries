use async_graphql::*;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use super::wallets;
use crate::AppContext;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize, SimpleObject)]
#[sea_orm(table_name = "treasuries")]
#[graphql(complex, concrete(name = "Treasury", params()))]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub vault_id: String,
    pub created_at: DateTime,
}

#[ComplexObject]

impl Model {
    async fn wallets(&self, ctx: &Context<'_>) -> Result<Option<Vec<wallets::Model>>> {
        let AppContext { wallets_loader, .. } = ctx.data::<AppContext>()?;

        wallets_loader.load_one(self.id).await
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_one = "super::customer_treasuries::Entity")]
    CustomerTreasuries,
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

impl Related<super::customer_treasuries::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CustomerTreasuries.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
