use async_graphql::*;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use super::treasuries;
use crate::AppContext;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "project_treasuries")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub project_id: Uuid,
    #[sea_orm(unique)]
    pub treasury_id: Uuid,
    pub created_at: DateTimeWithTimeZone,
}

#[Object(name = "ProjectTreasury")]
impl Model {
    async fn project_id(&self) -> &Uuid {
        &self.project_id
    }

    async fn treasury_id(&self) -> &Uuid {
        &self.treasury_id
    }

    async fn created_at(&self) -> &DateTimeWithTimeZone {
        &self.created_at
    }

    async fn treasury(&self, ctx: &Context<'_>) -> Result<Option<treasuries::Model>> {
        let AppContext { db, .. } = ctx.data::<AppContext>()?;
        let t = treasuries::Entity::find_by_id(self.treasury_id)
            .one(db.get())
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
