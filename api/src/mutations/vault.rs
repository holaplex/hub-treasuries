use std::sync::Arc;

use async_graphql::{self, Context, InputObject, Object, Result};
use fireblocks::objects::vault::VaultAccount;
use sea_orm::{prelude::*, Set};
use uuid::Uuid;

use crate::{
    entities::{organizations, organizations::ActiveModel, owners},
    UserID,
};

#[derive(Default)]
pub struct Mutation;

#[Object(name = "VaultMutation")]
impl Mutation {
    /// Res
    ///
    /// # Errors
    /// This function fails if ...
    pub async fn create_vault(
        &self,
        ctx: &Context<'_>,
        project_id: String,
    ) -> Result<VaultAccount> {
       
        let db = &**ctx.data::<Arc<DatabaseConnection>>()?;

        let id = Uuid::parse_str(&project_id)?;

        let org = ActiveModel::from(input).insert(db).await?;

        let owner = owners::ActiveModel {
            user_id: Set(user_id),
            organization_id: Set(org.id),
            ..Default::default()
        };

        owner.insert(db).await?;

        Ok(org)
    }
}

#[derive(InputObject)]
pub struct CreateOrganizationInput {
    pub name: String,
}

impl From<CreateOrganizationInput> for ActiveModel {
    fn from(val: CreateOrganizationInput) -> Self {
        Self {
            name: Set(val.name),
            ..Default::default()
        }
    }
}
