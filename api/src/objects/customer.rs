use async_graphql::{ComplexObject, Context, Result, SimpleObject};
use hub_core::uuid::Uuid;

use crate::{
    entities::{
        treasuries,
        wallets::{self, AssetType},
    },
    AppContext,
};

#[derive(Debug, Clone, SimpleObject)]
#[graphql(complex)]
pub struct Customer {
    pub id: Uuid,
}

#[ComplexObject]
impl Customer {
    /// Returns all the wallet addresses associated with the customer. The blockchain of the address is not included and they are in no particular order. In the future, the blockchain may be indicated with a pattern of {blockchain}:{address}.
    /// This field returns null when there is no treasury assigned to the customer yet.
    pub async fn addresses(&self, ctx: &Context<'_>) -> Result<Option<Vec<String>>> {
        let AppContext {
            customer_wallet_addresses_loader,
            ..
        } = ctx.data::<AppContext>()?;

        customer_wallet_addresses_loader.load_one(self.id).await
    }

    /// The treasury assigned to the customer, which contains the customer's wallets.
    pub async fn treasury(&self, ctx: &Context<'_>) -> Result<Option<treasuries::Model>> {
        let AppContext {
            customer_treasury_loader,
            ..
        } = ctx.data::<AppContext>()?;

        customer_treasury_loader.load_one(self.id).await
    }

    pub async fn wallet(
        &self,
        ctx: &Context<'_>,
        asset_id: Option<AssetType>,
    ) -> Result<Option<Vec<wallets::Model>>> {
        let AppContext {
            customer_treasury_wallet_loader,
            ..
        } = ctx.data::<AppContext>()?;

        let mut wallets = customer_treasury_wallet_loader.load_one(self.id).await?;

        if let Some(asset_id) = asset_id {
            wallets = wallets.clone().map(|w| {
                w.iter()
                    .filter(|wallet| wallet.asset_id == asset_id)
                    .cloned()
                    .collect::<Vec<_>>()
            });
        };

        Ok(wallets)
    }
}
