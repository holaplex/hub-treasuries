//! `SeaORM` Entity. Generated by sea-orm-codegen 0.10.5

use std::sync::Arc;

use async_graphql::{Context, Object, Result};
use sea_orm::entity::prelude::*;

use super::treasuries;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "project_treasuries")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub project_id: Uuid,
    #[sea_orm(unique)]
    pub treasury_id: Uuid,
    pub created_at: DateTime,
}

#[Object]
impl Model {
    async fn project_id(&self) -> &Uuid {
        &self.project_id
    }

    async fn treasury_id(&self) -> &Uuid {
        &self.treasury_id
    }

    async fn created_at(&self) -> &DateTime {
        &self.created_at
    }

    async fn treasury(&self, ctx: &Context<'_>) -> Result<Option<treasuries::Model>> {
        let db = &**ctx.data::<Arc<DatabaseConnection>>()?;
        let t = treasuries::Entity::find_by_id(self.treasury_id)
            .one(db)
            .await?;

        // let fireblocks = ctx.data::<FireblocksClient>()?;
        // let vault = fireblocks.get_vault(vault_id).await?;

        Ok(t)
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::treasuries::Entity",
        from = "Column::TreasuryId",
        to = "super::treasuries::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Treasuries,
}

impl Related<super::treasuries::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Treasuries.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
