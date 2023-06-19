use std::str::FromStr;

use async_graphql::{Enum, Result, SimpleObject};
use hub_core::anyhow::{anyhow, Error};
use sea_orm::entity::prelude::*;

const SOL: &str = "SOL";
const MATIC: &str = "MATIC";
const ETH: &str = "ETH";

/// Fireblocks-defined blockchain identifiers.
#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum AssetType {
    /// Mainnet Solana
    #[graphql(name = "SOL")]
    #[sea_orm(num_value = 0)]
    Solana,
    /// Mainnet Polygon
    #[graphql(name = "MATIC")]
    #[sea_orm(num_value = 3)]
    Matic,
    /// Ethereum Mainnet
    #[graphql(name = "ETH")]
    #[sea_orm(num_value = 5)]
    Eth,
}

impl AssetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Solana => SOL,
            Self::Matic => MATIC,
            Self::Eth => ETH,
        }
    }
}

impl From<AssetType> for String {
    fn from(value: AssetType) -> Self {
        value.as_str().to_string()
    }
}

impl From<AssetType> for i32 {
    fn from(value: AssetType) -> Self {
        match value {
            AssetType::Solana => 0,
            AssetType::Matic => 3,
            AssetType::Eth => 5,
        }
    }
}

impl FromStr for AssetType {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            SOL => Ok(Self::Solana),
            MATIC => Ok(Self::Matic),
            ETH => Ok(Self::Eth),
            &_ => Err(anyhow!("unsupported  asset_type")),
        }
    }
}

/// A blockchain wallet.
/// # Description
/// A blockchain wallet is a digital wallet that allows users to securely store, manage, and transfer their cryptocurrencies or other digital assets on a blockchain network.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, SimpleObject)]
#[sea_orm(table_name = "wallets")]
#[graphql(concrete(name = "Wallet", params()))]
pub struct Model {
    pub treasury_id: Uuid,
    /// The wallet address.
    pub address: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub removed_at: Option<DateTimeWithTimeZone>,
    pub created_by: Uuid,
    /// The wallet's associated blockchain.
    pub asset_id: AssetType,
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub deduction_id: Option<Uuid>,
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
