use async_graphql::{Context, Object, Result};
use hub_core::uuid::Uuid;

use crate::objects::Customer;

#[derive(Default)]
pub struct Query;

#[Object(name = "CustomerQuery")]
impl Query {
    /// Res
    ///
    /// # Errors
    /// This function fails if ...
    #[graphql(entity)]
    async fn find_customer_by_id(
        &self,
        _ctx: &Context<'_>,
        #[graphql(key)] id: Uuid,
    ) -> Result<Customer> {
        Ok(Customer { id })
    }
}
