use std::str::FromStr;

use async_graphql::{self, Context, Error, Object, Result};
use entities::prelude::*;
use fireblocks::{
    objects::vault::{CreateVault, CreateVaultAssetResponse, CreateVaultWallet, VaultAccount},
    Client as FireblocksClient,
};
use sea_orm::{prelude::*, Set};

use crate::{
    entities::{self, project_treasuries, treasuries, wallets},
    AppContext, UserID,
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
        let AppContext { db, user_id, .. } = ctx.data::<AppContext>()?;
        let fireblocks = ctx.data::<FireblocksClient>()?;
        let UserID(id) = user_id;

        let user_id = id.ok_or_else(|| Error::new("X-USER-ID header not found"))?;

        let create_vault = CreateVault {
            name: project_id.to_string(),
            hidden_on_ui: None,
            customer_ref_id: Some(user_id.to_string()),
            auto_fuel: Some(false),
        };

        let vault = fireblocks.create_vault(create_vault).await?;

        let treasury = treasuries::ActiveModel {
            vault_id: Set(vault.id.clone()),
            ..Default::default()
        };

        let treasury: treasuries::Model = treasury.clone().insert(db.get()).await?;

        let project_treasuries_active_model = project_treasuries::ActiveModel {
            project_id: Set(Uuid::parse_str(&project_id)?),
            treasury_id: Set(treasury.id),
            ..Default::default()
        };

        project_treasuries_active_model.insert(db.get()).await?;

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

        let AppContext { db, user_id, .. } = ctx.data::<AppContext>()?;
        let fireblocks = ctx.data::<FireblocksClient>()?;
        let UserID(id) = user_id;

        let user_id = id.ok_or_else(|| Error::new("X-USER-ID header not found"))?;

        // insert treasury to get the treasury id

        let treasury_id = Uuid::from_str(&treasury_id)?;

        let treasury = Treasuries::find_by_id(treasury_id)
            .one(db.get())
            .await?
            .ok_or_else(|| Error::new("failed to load treasury"))?;

        let vault_asset = fireblocks
            .create_vault_wallet(treasury.vault_id.clone(), asset_id, CreateVaultWallet {
                eos_account_name: None,
            })
            .await?;

        let v = vault_asset.clone();

        let active_model = wallets::ActiveModel {
            treasury_id: Set(treasury_id),
            asset_id: Set(vault_asset.id),
            address: Set(vault_asset.address),
            legacy_address: Set(vault_asset.legacy_address),
            tag: Set(vault_asset.tag),
            created_by: Set(user_id),
            ..Default::default()
        };
        active_model.insert(db.get()).await?;

        Ok(v)
    }
}
