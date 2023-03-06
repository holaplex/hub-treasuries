use std::str::FromStr;

use async_graphql::{self, Context, Error, InputObject, Object, Result, SimpleObject};
use entities::prelude::*;
use fireblocks::{objects::vault::CreateVaultWallet, Client as FireblocksClient};
use hub_core::producer::Producer;
use sea_orm::{prelude::*, JoinType, QuerySelect, Set};

use crate::{
    entities::{
        self, customer_treasuries, project_treasuries,
        treasuries::{self, TreasuryAndProjectIds},
        wallets::{self, AssetType},
    },
    proto::{
        treasury_events::{self, Blockchain, CustomerWallet, ProjectWallet},
        TreasuryEventKey, TreasuryEvents,
    },
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
    pub async fn create_treasury_wallet(
        &self,
        ctx: &Context<'_>,
        input: CreateTreasuryWalletInput,
    ) -> Result<CreateTreasuryWalletPayload> {
        let AppContext { db, user_id, .. } = ctx.data::<AppContext>()?;
        let fireblocks = ctx.data::<FireblocksClient>()?;
        let producer = ctx.data::<Producer<TreasuryEvents>>()?;

        let UserID(id) = user_id;
        let CreateTreasuryWalletInput {
            treasury_id,
            asset_type,
        } = input;

        let user_id = id.ok_or_else(|| Error::new("X-USER-ID header not found"))?;

        // insert treasury to get the treasury id

        let treasury = Treasuries::find_by_id(treasury_id)
            .select_only()
            .column(treasuries::Column::Id)
            .column(treasuries::Column::VaultId)
            .column_as(
                customer_treasuries::Column::ProjectId,
                "customer_project_id",
            )
            .column(customer_treasuries::Column::CustomerId)
            .column_as(project_treasuries::Column::ProjectId, "project_project_id")
            .join(
                JoinType::LeftJoin,
                treasuries::Relation::ProjectTreasuries.def(),
            )
            .join(
                JoinType::LeftJoin,
                treasuries::Relation::CustomerTreasuries.def(),
            )
            .into_model::<TreasuryAndProjectIds>()
            .one(db.get())
            .await?
            .ok_or_else(|| Error::new("failed to load treasury"))?;

        if treasury.customer_project_id.is_none() || treasury.project_project_id.is_none() {
            return Err(Error::new("customer or project treasury not found"));
        }

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
            asset_id: Set(asset_type),
            address: Set(vault_asset.address.clone()),
            legacy_address: Set(vault_asset.legacy_address),
            tag: Set(vault_asset.tag),
            created_by: Set(user_id),
            ..Default::default()
        };

        let wallet = active_model.insert(db.get()).await?;

        let event = if let (Some(project_id), Some(customer_id)) =
            (treasury.customer_project_id, treasury.customer_id)
        {
            Some(treasury_events::Event::CustomerWalletCreated(
                CustomerWallet {
                    project_id: project_id.to_string(),
                    customer_id: customer_id.to_string(),
                    blockchain: AssetType::from_str(&vault_asset.id)?.into(),
                },
            ))
        } else if let Some(project_id) = treasury.project_project_id {
            Some(treasury_events::Event::ProjectWalletCreated(
                ProjectWallet {
                    project_id: project_id.to_string(),
                    wallet_address: vault_asset.address.clone(),
                    blockchain: AssetType::from_str(&vault_asset.id)?.into(),
                },
            ))
        } else {
            None
        };

        let event = TreasuryEvents { event };
        let key = TreasuryEventKey {
            id: treasury.id.to_string(),
        };

        producer.send(Some(&event), Some(&key)).await?;

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

impl From<AssetType> for Blockchain {
    fn from(value: AssetType) -> Self {
        match value {
            AssetType::Solana | AssetType::SolanaTest => Blockchain::Solana,
            AssetType::MaticTest | AssetType::Matic => Blockchain::Polygon,
            AssetType::EthTest | AssetType::Eth => Blockchain::Ethereum,
        }
    }
}
