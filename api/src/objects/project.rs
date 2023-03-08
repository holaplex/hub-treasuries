use async_graphql::{ComplexObject, Context, Result, SimpleObject};
use hub_core::uuid::Uuid;

use crate::{entities::treasuries, AppContext};

#[derive(Debug, Clone, SimpleObject)]
#[graphql(complex)]
pub struct Project {
    pub id: Uuid,
}

#[ComplexObject]
impl Project {
    pub async fn treasury(&self, ctx: &Context<'_>) -> Result<Option<treasuries::Model>> {
        let AppContext {
            project_treasury_loader,
            ..
        } = ctx.data::<AppContext>()?;

        project_treasury_loader.load_one(self.id).await
    }
}
