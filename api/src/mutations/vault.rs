use std::str::FromStr;

use async_graphql::{Context, Error, InputObject, Object, Result, SimpleObject};
use fireblocks::{objects::vault::CreateVaultWallet, Client as FireblocksClient};
use hub_core::producer::Producer;
use sea_orm::{prelude::*, JoinType, QuerySelect, Set};

use crate::{
    entities::{customer_treasuries, treasuries, wallets},
    proto::{treasury_events, TreasuryEventKey, TreasuryEvents},
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
    pub async fn create_customer_wallet(
        &self,
        ctx: &Context<'_>,
        input: CreateCustomerWalletInput,
    ) -> Result<CreateCustomerWalletPayload> {
        let AppContext { db, user_id, .. } = ctx.data::<AppContext>()?;
        let fireblocks = ctx.data::<FireblocksClient>()?;
        let conn = db.get();
        let producer = ctx.data::<Producer<TreasuryEvents>>()?;

        let UserID(id) = user_id;
        let CreateCustomerWalletInput {
            customer,
            asset_type,
        } = input;

        let user_id = id.ok_or_else(|| Error::new("X-USER-ID header not found"))?;

        let (customer_treasury, treasury) = customer_treasuries::Entity::find()
            .join(
                JoinType::InnerJoin,
                customer_treasuries::Relation::Treasury.def(),
            )
            .filter(customer_treasuries::Column::CustomerId.eq(customer))
            .select_also(treasuries::Entity)
            .one(conn)
            .await?
            .ok_or_else(|| Error::new("customer treasury not found"))?;

        let treasury = treasury.ok_or_else(|| Error::new("treasury not found"))?;

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
            treasury_id: Set(treasury.id),
            asset_id: Set(asset_type),
            address: Set(vault_asset.address.clone()),
            legacy_address: Set(vault_asset.legacy_address),
            tag: Set(vault_asset.tag),
            created_by: Set(user_id),
            ..Default::default()
        };

        let wallet = active_model.insert(conn).await?;

        let event = TreasuryEvents {
            event: Some(treasury_events::Event::CustomerWalletCreated(
                treasury_events::CustomerWallet {
                    project_id: customer_treasury.project_id.to_string(),
                    customer_id: customer_treasury.customer_id.to_string(),
                    blockchain: wallets::AssetType::from_str(&vault_asset.id)?.into(),
                },
            )),
        };
        let key = TreasuryEventKey {
            id: treasury.id.to_string(),
        };

        producer.send(Some(&event), Some(&key)).await?;

        Ok(CreateCustomerWalletPayload { wallet })
    }
}

#[derive(InputObject, Clone, Debug)]
pub struct CreateCustomerWalletInput {
    pub customer: Uuid,
    pub asset_type: wallets::AssetType,
}

#[derive(SimpleObject, Clone, Debug)]
pub struct CreateCustomerWalletPayload {
    pub wallet: wallets::Model,
}

impl From<wallets::AssetType> for treasury_events::Blockchain {
    fn from(value: wallets::AssetType) -> Self {
        match value {
            wallets::AssetType::Solana | wallets::AssetType::SolanaTest => {
                treasury_events::Blockchain::Solana
            },
            wallets::AssetType::MaticTest | wallets::AssetType::Matic => {
                treasury_events::Blockchain::Polygon
            },
            wallets::AssetType::EthTest | wallets::AssetType::Eth => {
                treasury_events::Blockchain::Ethereum
            },
        }
    }
}
