use std::str::FromStr;

use async_graphql::{Enum, Result, SimpleObject};
use fireblocks::assets::{ETH, ETH_TEST, MATIC, MATIC_POLYGON, MATIC_TEST, SOL, SOL_TEST};
use hub_core::{credits::Blockchain, thiserror};
use sea_orm::entity::prelude::*;

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

impl From<AssetType> for Blockchain {
    fn from(value: AssetType) -> Self {
        match value {
            AssetType::Solana => Blockchain::Solana,
            AssetType::Matic => Blockchain::Polygon,
            AssetType::Eth => Blockchain::Ethereum,
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
            AssetType::Solana => 1,
            AssetType::Matic => 2,
            AssetType::Eth => 3,
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid asset type {0:?}")]
pub struct TryIntoAssetTypeError(String);

impl FromStr for AssetType {
    type Err = TryIntoAssetTypeError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            SOL | SOL_TEST => Ok(Self::Solana),
            MATIC_POLYGON | MATIC_TEST => Ok(Self::Matic),
            ETH | ETH_TEST => Ok(Self::Eth),
            s => Err(TryIntoAssetTypeError(s.into())),
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

impl ActiveModelBehavior for ActiveModel {
    hub_core::before_save_evm_addrs!(address);
}
