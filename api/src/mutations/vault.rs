use std::{str::FromStr, sync::Arc};

use async_graphql::{self, Context, Error, Object, Result};
use fireblocks::{
    client::FireblocksClient,
    objects::vault::{CreateVault, CreateVaultAssetResponse, CreateVaultWallet, VaultAccount},
};
use models::prelude::*;
use sea_orm::{prelude::*, Set};
use uuid::Uuid;

use crate::{
    models::{self, project_treasuries, treasuries, wallets},
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
    pub async fn create_vault(&self, ctx: &Context<'_>, project_id: Uuid) -> Result<VaultAccount> {
        let db = &**ctx.data::<Arc<DatabaseConnection>>()?;
        let fireblocks = &**ctx.data::<Arc<FireblocksClient>>()?;

        // insert treasury to get the treasury id

        let mut treasury = treasuries::ActiveModel {
            vault_id: Set(Uuid::new_v4().to_string()),
            ..Default::default()
        };

        treasury.clone().insert(db).await?;

        let treasury_id = treasury.id.clone().unwrap().to_string();

        let create_vault = CreateVault {
            name: treasury_id,
            hidden_on_ui: None,
            customer_ref_id: None,
            auto_fuel: None,
        };

        let vault = fireblocks.create_vault(create_vault).await?;

        treasury.vault_id = Set(vault.id.clone());

        treasury.clone().update(db).await?;

        // insert into project_treasuries table

        let project_treasuries_active_model = project_treasuries::ActiveModel {
            project_id: Set(project_id),
            treasury_id: treasury.id,
            ..Default::default()
        };

        project_treasuries_active_model.insert(db).await?;

        Ok(vault)
    }

    /// Res
    ///
    /// # Errors
    /// This function fails if ...
    pub async fn create_treasury_wallet(
        &self,
        ctx: &Context<'_>,
        treasury_id: String,
        asset_id: String,
    ) -> Result<CreateVaultAssetResponse> {
        // AssetID would be enum for polygon/solana
        // Reterive assets endpoint

        let db = &**ctx.data::<Arc<DatabaseConnection>>()?;
        let fireblocks = &**ctx.data::<Arc<FireblocksClient>>()?;
        let UserID(id) = ctx.data::<UserID>()?;

        let user_id = Uuid::parse_str(id)?;

        // insert treasury to get the treasury id

        let treasury_id = Uuid::from_str(&treasury_id)?;

        let treasury = Treasuries::find_by_id(treasury_id)
            .one(db)
            .await?
            .ok_or_else(|| Error::new("failed to load treasury"))?;

        let vault = fireblocks
            .create_vault_wallet(treasury.vault_id.clone(), asset_id, CreateVaultWallet {
                eos_account_name: None,
            })
            .await?;

        if vault.id != treasury.vault_id {
            return Err(Error::new(
                "vault.id from fireblocks response does not match database treasury vault",
            ));
        }

        let v = vault.clone();

        let active_model = wallets::ActiveModel {
            treasury_id: Set(treasury_id),
            address: Set(vault.address),
            legacy_address: Set(vault.legacy_address),
            tag: Set(vault.tag),
            user_id: Set(user_id),
            ..Default::default()
        };
        active_model.insert(db).await?;

        Ok(v)
    }
}
