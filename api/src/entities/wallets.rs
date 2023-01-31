use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, poem_openapi::Object)]
#[sea_orm(table_name = "wallets")]
#[oai(rename = "Wallet")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub treasury_id: Uuid,
    pub asset_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub address: String,
    pub legacy_address: String,
    pub tag: String,
    pub created_at: DateTime,
    pub removed_at: Option<DateTime>,
    pub created_by: Uuid,
}

// #[Object(name = "Wallet")]
// impl Model {
//     async fn treasury_id(&self) -> &Uuid {
//         &self.treasury_id
//     }

//     async fn asset_id(&self) -> &str {
//         &self.asset_id
//     }

//     async fn address(&self) -> &str {
//         &self.address
//     }

//     async fn legacy_address(&self) -> &str {
//         &self.legacy_address
//     }

//     async fn tag(&self) -> &str {
//         &self.tag
//     }

//     async fn created_at(&self) -> &DateTime {
//         &self.created_at
//     }

//     async fn removed_at(&self) -> Option<DateTime> {
//         self.removed_at
//     }

//     async fn created_by(&self) -> &Uuid {
//         &self.created_by
//     }

//     async fn balance(&self, ctx: &Context<'_>) -> Result<Vec<VaultAsset>> {
//         let fireblocks = ctx.data::<FireblocksClient>()?;
//         let AppContext { db, .. } = ctx.data::<AppContext>()?;

//         let res = treasuries::Entity::find_by_id(self.treasury_id)
//             .one(db.get())
//             .await?;

//         let t = res.ok_or_else(|| Error::new("failed to get treasury"))?;

//         let v = fireblocks.get_vault(t.vault_id.clone()).await?;

//         Ok(v.assets)
//     }
// }

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
