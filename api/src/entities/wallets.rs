use std::str::FromStr;

use async_graphql::{Enum, Result, SimpleObject};
use hub_core::anyhow::{anyhow, Error};
use sea_orm::entity::prelude::*;

const SOL: &str = "SOL";
const SOL_TEST: &str = "SOL_TEST";

#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum AssetType {
    #[graphql(name = "SOL")]
    #[sea_orm(num_value = 0)]
    Solana,
    #[graphql(name = "SOL_TEST")]
    #[sea_orm(num_value = 1)]
    SolanaTest,
}

impl From<AssetType> for i32 {
    fn from(value: AssetType) -> Self {
        match value {
            AssetType::Solana => 0,
            AssetType::SolanaTest => 1,
        }
    }
}

impl From<AssetType> for String {
    fn from(value: AssetType) -> Self {
        match value {
            AssetType::Solana => SOL.to_string(),
            AssetType::SolanaTest => SOL_TEST.to_string(),
        }
    }
}

impl FromStr for AssetType {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            SOL => Ok(Self::Solana),
            SOL_TEST => Ok(Self::SolanaTest),
            &_ => Err(anyhow!("unsupported  asset_type")),
        }
    }
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, SimpleObject)]
#[sea_orm(table_name = "wallets")]
#[graphql(concrete(name = "Wallet", params()))]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub treasury_id: Uuid,
    pub asset_id: AssetType,
    #[sea_orm(primary_key, auto_increment = false)]
    pub address: String,
    pub legacy_address: String,
    pub tag: String,
    pub created_at: DateTime,
    pub removed_at: Option<DateTime>,
    pub created_by: Uuid,
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
