use std::{fmt, sync::Arc};

use async_graphql::{self, Context, Enum, Object, Result};
use fireblocks::{
    client::FireblocksClient,
    objects::vault::{QueryVaultAccounts, VaultAccount, VaultAccountsPagedResponse, VaultAsset},
};

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
}
