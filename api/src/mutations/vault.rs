use async_graphql::{Context, Enum, Error, InputObject, Object, Result, SimpleObject};
use fireblocks::{objects::vault::CreateVaultWallet, Client as FireblocksClient};
use sea_orm::{prelude::*, Set};

use crate::{
    entities::{treasuries, wallets},
    AppContext, UserID,
};

#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetType {
    #[graphql(name = "SOL")]
    Solana,
    #[graphql(name = "SOL_TEST")]
    SolanaTest,
}

impl From<AssetType> for String {
    fn from(value: AssetType) -> Self {
        match value {
            AssetType::Solana => "SOL".to_string(),
            AssetType::SolanaTest => "SOL_TEST".to_string(),
        }
    }
}

#[derive(Default)]
pub struct Mutation;

#[Object(name = "VaultMutation")]
impl Mutation {
    /// Res
    ///
    /// # Errors
    /// This function fails if ...
    pub async fn create_treasury_wallet(
        &self,
        ctx: &Context<'_>,
        input: CreateTreasuryWalletInput,
    ) -> Result<CreateTreasuryWalletPayload> {
        let AppContext { db, user_id, .. } = ctx.data::<AppContext>()?;
        let fireblocks = ctx.data::<FireblocksClient>()?;
        let UserID(id) = user_id;
        let CreateTreasuryWalletInput {
            treasury_id,
            asset_type,
        } = input;

        let user_id = id.ok_or_else(|| Error::new("X-USER-ID header not found"))?;

        let treasury = treasuries::Entity::find()
            .filter(treasuries::Column::Id.eq(treasury_id))
            .one(db.get())
            .await?
            .ok_or_else(|| Error::new("failed to load treasury"))?;

        let vault_asset = fireblocks
            .create_vault_wallet(
                treasury.vault_id.clone(),
                asset_type.into(),
                CreateVaultWallet {
                    eos_account_name: None,
                },
            )
            .await?;

        let active_model = wallets::ActiveModel {
            treasury_id: Set(treasury_id),
            asset_id: Set(vault_asset.id),
            address: Set(vault_asset.address),
            legacy_address: Set(vault_asset.legacy_address),
            tag: Set(vault_asset.tag),
            created_by: Set(user_id),
            ..Default::default()
        };
        let wallet = active_model.insert(db.get()).await?;

        Ok(CreateTreasuryWalletPayload { wallet })
    }
}

#[derive(InputObject, Clone, Debug)]
pub struct CreateTreasuryWalletInput {
    pub treasury_id: Uuid,
    pub asset_type: AssetType,
}

#[derive(SimpleObject, Clone, Debug)]
pub struct CreateTreasuryWalletPayload {
    pub wallet: wallets::Model,
}
