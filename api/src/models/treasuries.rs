//! `SeaORM` Entity. Generated by sea-orm-codegen 0.10.5

use std::sync::Arc;

use async_graphql::{Context, Object, Result};
use fireblocks::{client::FireblocksClient, objects::vault::VaultAsset};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "treasuries")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub vault_id: String,
    pub created_at: DateTime,
}

#[Object]
impl Model {
    async fn id(&self) -> &Uuid {
        &self.id
    }

    async fn vault_id(&self) -> &str {
        &self.vault_id
    }

    async fn created_at(&self) -> &DateTime {
        &self.created_at
    }

    async fn wallets(&self, ctx: &Context<'_>) -> Result<Vec<VaultAsset>> {
        let fireblocks = &**ctx.data::<Arc<FireblocksClient>>()?;

        let v = fireblocks.get_vault(self.vault_id.clone()).await?;

        // let t = wallets::Entity::find()
        //     .filter(wallets::Column::TreasuryId.eq(self.id))
        //     .all(db)
        //     .await?;

        Ok(v.assets)
    }
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
