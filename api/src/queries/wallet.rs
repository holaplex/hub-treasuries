use async_graphql::{Context, Error, Object, Result};
use hub_core::util::ValidateAddress;

use crate::{entities::wallets::Model, AppContext};
#[derive(Debug, Clone, Copy, Default)]
pub struct Query;

#[Object(name = "WalletQuery")]
impl Query {
    /// Entity resolver for Wallet.
    /// Retrieves a Wallet model by its blockchain address.
    ///
    /// This method is marked as an entity resolver in a federated GraphQL schema,
    /// allowing multiple subgraphs to contribute fields to the same object type
    ///
    /// # Errors
    /// This function fails if the `AppContext` cannot be accessed,
    /// the address provided is not a valid blockchain address
    /// or fails to load from the database.
    #[graphql(entity)]
    async fn find_wallet_by_address(
        &self,
        ctx: &Context<'_>,
        #[graphql(key)] address: String,
    ) -> Result<Option<Model>> {
        self.wallet(ctx, address).await
    }

    /// Query to find a `Wallet` by its blockchain address.
    ///
    /// # Errors
    /// This function fails if the `AppContext` cannot be accessed,
    /// the address provided is not a valid blockchain address
    /// or fails to load from the database.
    async fn wallet(&self, ctx: &Context<'_>, address: String) -> Result<Option<Model>> {
        if !ValidateAddress::is_blockchain_address(&address) {
            return Err(Error::new("Invalid address"));
        }

        let AppContext { wallet_loader, .. } = ctx.data::<AppContext>()?;

        wallet_loader.load_one(address).await
    }
}
