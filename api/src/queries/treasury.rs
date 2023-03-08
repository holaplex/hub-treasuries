use async_graphql::{Context, Object, Result};
use hub_core::uuid::Uuid;

use crate::{entities::treasuries::Model, AppContext};

#[derive(Default)]
pub struct Query;

#[Object(name = "TreasuryQuery")]
impl Query {
    /// Res
    ///
    /// # Errors
    /// This function fails if ...
    #[graphql(entity)]
    async fn find_treasury_by_id(
        &self,
        ctx: &Context<'_>,
        #[graphql(key)] id: Uuid,
    ) -> Result<Option<Model>> {
        let AppContext {
            treasury_loader, ..
        } = ctx.data::<AppContext>()?;

        treasury_loader.load_one(id).await
    }
}
