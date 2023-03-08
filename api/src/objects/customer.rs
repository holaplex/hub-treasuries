use async_graphql::{ComplexObject, Context, Result, SimpleObject};
use hub_core::uuid::Uuid;

use crate::{entities::treasuries, AppContext};

#[derive(Debug, Clone, SimpleObject)]
#[graphql(complex)]
pub struct Customer {
    pub id: Uuid,
}

#[ComplexObject]
impl Customer {
    pub async fn treasury(&self, ctx: &Context<'_>) -> Result<Option<treasuries::Model>> {
        let AppContext {
            customer_treasury_loader,
            ..
        } = ctx.data::<AppContext>()?;

        customer_treasury_loader.load_one(self.id).await
    }
}
