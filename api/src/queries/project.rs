use async_graphql::{Context, Object, Result};
use hub_core::uuid::Uuid;

use crate::objects::Project;

#[derive(Default)]
pub struct Query;

#[Object(name = "ProjectQuery")]
impl Query {
    /// Res
    ///
    /// # Errors
    /// This function fails if ...
    #[graphql(entity)]
    async fn find_project_by_id(
        &self,
        _ctx: &Context<'_>,
        #[graphql(key)] id: Uuid,
    ) -> Result<Project> {
        Ok(Project { id })
    }
}
