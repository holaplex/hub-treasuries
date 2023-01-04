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
    pub async fn create_vault(
        &self,
        ctx: &Context<'_>,
        project_id: String,
    ) -> Result<VaultAccount> {
        let db = &**ctx.data::<Arc<DatabaseConnection>>()?;
        let fireblocks = &**ctx.data::<Arc<FireblocksClient>>()?;
        let UserID(id) = ctx.data::<UserID>()?;

        let create_vault = CreateVault {
            name: project_id.to_string(),
            hidden_on_ui: None,
            customer_ref_id: Some(id.to_string()),
            auto_fuel: Some(false),
        };

        let vault = fireblocks.create_vault(create_vault).await?;

        let treasury = treasuries::ActiveModel {
            vault_id: Set(vault.id.clone()),
            ..Default::default()
        };

        let treasury: treasuries::Model = treasury.clone().insert(db).await?;

        let project_treasuries_active_model = project_treasuries::ActiveModel {
            project_id: Set(Uuid::parse_str(&project_id)?),
            treasury_id: Set(treasury.id),
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
            created_by: Set(user_id),
            ..Default::default()
        };
        active_model.insert(db).await?;

        Ok(v)
    }
}
