use std::{fmt, sync::Arc};

use async_graphql::{self, Context, Enum, Object, Result};
use fireblocks::{
    client::FireblocksClient,
    objects::vault::{QueryVaultAccounts, VaultAccount, VaultAccountsPagedResponse, VaultAsset},
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::models::project_treasuries;

#[derive(Enum, Debug, Copy, Clone, Eq, PartialEq)]
pub enum OrderBy {
    #[graphql(name = "Ascending")]
    Asc,
    #[graphql(name = "Descending")]
    Desc,
}

impl fmt::Display for OrderBy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Default for OrderBy {
    fn default() -> Self {
        Self::Desc
    }
}

#[derive(Default)]
pub struct Query;

#[Object(name = "VaultQuery")]
impl Query {
    /// Res
    ///
    /// # Errors
    /// This function fails if ...
    async fn vaults(
        &self,
        ctx: &Context<'_>,
        asset_id: Option<u64>,
        limit: Option<u64>,
        order_by: Option<OrderBy>,
    ) -> Result<VaultAccountsPagedResponse> {
        let fireblocks = &**ctx.data::<Arc<FireblocksClient>>()?;

        let vaults = fireblocks
            .get_vaults(QueryVaultAccounts {
                name_prefix: None,
                name_suffix: None,
                min_amount_threshold: None,
                asset_id,
                order_by: order_by.unwrap_or_default().to_string(),
                limit: limit.unwrap_or(500),
                before: None,
                after: None,
                max_bip44_address_index_used: 966,
                max_bip44_change_address_index_used: 20,
            })
            .await?;

        Ok(vaults)
    }

    async fn vault(&self, ctx: &Context<'_>, vault_id: String) -> Result<VaultAccount> {
        let fireblocks = &**ctx.data::<Arc<FireblocksClient>>()?;

        let vault = fireblocks.get_vault(vault_id).await?;

        Ok(vault)
    }

    async fn vault_assets(&self, ctx: &Context<'_>) -> Result<Vec<VaultAsset>> {
        let fireblocks = &**ctx.data::<Arc<FireblocksClient>>()?;

        let vault = fireblocks.vault_assets().await?;

        Ok(vault)
    }

    async fn project_treasuries(
        &self,
        ctx: &Context<'_>,
        project_id: Uuid,
    ) -> Result<Vec<project_treasuries::Model>> {
        let db = ctx.data::<DatabaseConnection>()?;
        let t = project_treasuries::Entity::find()
            .filter(project_treasuries::Column::ProjectId.eq(project_id))
            .all(db)
            .await?;

        Ok(t)
    }
}

// #[derive(Enum, Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
// pub enum Blockchain {
//     Polygon,
//     Solana,
// }

// #[derive(Union,Debug, Clone, Serialize, Deserialize,)]
// pub enum Currency {
//     Lamports(Lamports),
//     Matic(Matic),
// }

// #[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
// pub struct Lamports {
//     value: u64,
// }

// #[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
// pub struct Matic {
//     value: f64,
// }

// #[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
// pub struct SolanaWallet {
//     pub resource: String,
//     pub address: String,
//     pub balance: Currency,
//     pub chain: Blockchain,
// }

// #[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
// pub struct PolygonWallet {
//     pub resource: String,
//     pub address: String,
//     pub balance: Currency,
//     pub chain: Blockchain,
// }

// #[derive(Interface)]
// #[graphql(
//     field(name = "resource", type = "String"),
//     field(name = "address", type = "String"),
//     field(name = "balance", type = "&Currency"),
//     field(name = "chain", type = "&Blockchain")
// )]
// pub enum Wallet {
//     SolanaWallet(SolanaWallet),
//     PolygonWallet(PolygonWallet),
// }

// // #[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
// // pub struct ProjectTreasury {
// //     resource: String,
// //     project: Project!
// //     wallets(limit: Int = 25, offset: Int = 0): [Wallet!]
// //   }

// impl From<VaultAsset> for Wallet {
//     fn from(v : VaultAsset) -> Self {
//          const SOLANA_ASSET_ID: String = "SOL_TEST".to_string();
//          const POLYGON_ASSET_ID: String = "MATIC_TEST".to_string();

//         match v.id {
//             SOLANA_ASSET_ID => Wallet::SolanaWallet(SolanaWallet { resource: v.id, address:  v, balance: (), chain: () }),
//             POLYGON_ASSET_ID => Wallet::PolygonWallet(PolygonWallet { resource: (), address: (), balance: (), chain:)

//         }
//     }
// }
