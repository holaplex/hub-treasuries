use async_graphql::{ComplexObject, Context, Result, SimpleObject};
use hub_core::uuid::Uuid;

use crate::{
    entities::{treasuries, wallets},
    AppContext,
};

#[derive(Debug, Clone, SimpleObject)]
#[graphql(complex)]
pub struct Customer {
    pub id: Uuid,
}

#[ComplexObject]
impl Customer {
    /// The treasury assigned to the customer, which contains the customer's wallets.
    pub async fn treasury(&self, ctx: &Context<'_>) -> Result<Option<treasuries::Model>> {
        let AppContext {
            customer_treasury_loader,
            ..
        } = ctx.data::<AppContext>()?;

        customer_treasury_loader.load_one(self.id).await
    }

    pub async fn wallet(&self, ctx: &Context<'_>) -> Result<Option<wallets::Model>> {
        let AppContext {
            customer_treasury_wallet_loader,
            ..
        } = ctx.data::<AppContext>()?;

        customer_treasury_wallet_loader.load_one(self.id).await
    }
}
