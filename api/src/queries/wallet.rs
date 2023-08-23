use async_graphql::{Context, Error, Object, Result};
use hub_core::util::ValidateAddress;

use crate::{entities::wallets::Model, AppContext};
#[derive(Debug, Clone, Copy, Default)]
pub struct Query;

#[Object(name = "WalletQuery")]
impl Query {
    /// Res
    ///
    /// # Errors
    /// This function fails if ...
    #[graphql(entity)]
    async fn find_wallet_by_address(
        &self,
        ctx: &Context<'_>,
        #[graphql(key)] address: String,
    ) -> Result<Option<Model>> {
        if !ValidateAddress::is_blockchain_address(&address) {
            return Err(Error::new("Invalid address"));
        }

        let AppContext { wallet_loader, .. } = ctx.data::<AppContext>()?;

        wallet_loader.load_one(address).await
    }
}
