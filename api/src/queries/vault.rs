use std::sync::Arc;

use async_graphql::{self, Context, Object, Result};
use fireblocks::{
    client::FireblocksClient,
    objects::vault::{QueryVaultAccounts, VaultAccount, VaultAccountsPagedResponse},
};
use sea_orm::{prelude::*, QueryOrder};

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
        order_by: Option<String>,
    ) -> Result<VaultAccountsPagedResponse> {
        let fireblocks = &**ctx.data::<Arc<FireblocksClient>>()?;

        let vaults = fireblocks
            .get_vaults(QueryVaultAccounts {
                name_prefix: None,
                name_suffix: None,
                min_amount_threshold: None,
                asset_id,
                order_by: order_by.unwrap_or("DESC".to_string()),
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
}
