use async_graphql::{Context, Error, InputObject, Object, Result, SimpleObject};
use fireblocks::{objects::vault::CreateVaultWallet, Fireblocks};
use hub_core::{
    chrono::Utc,
    credits::{CreditsClient, DeductionErrorKind, TransactionId},
    producer::Producer,
};
use sea_orm::{prelude::*, JoinType, QuerySelect, Set};

use crate::{
    entities::{customer_treasuries, prelude::Wallets, treasuries, wallets},
    proto::{treasury_events, TreasuryEventKey, TreasuryEvents},
    Actions, AppContext,
};

#[derive(Default)]
pub struct Mutation;

#[Object(name = "VaultMutation")]
impl Mutation {
    /// Create a wallet for a customer and assign it to the customer's treasury account.
    ///
    /// # Errors
    /// The mutation will result in an error if it is unable to interact with the database or communicate with Fireblocks.
    pub async fn create_customer_wallet(
        &self,
        ctx: &Context<'_>,
        input: CreateCustomerWalletInput,
    ) -> Result<CreateCustomerWalletPayload> {
        let AppContext {
            db,
            user_id,
            organization_id,
            balance,
            ..
        } = ctx.data::<AppContext>()?;
        let fireblocks = ctx.data::<Fireblocks>()?;
        let credits = ctx.data::<CreditsClient<Actions>>()?;
        let conn = db.get();
        let producer = ctx.data::<Producer<TreasuryEvents>>()?;
        let CreateCustomerWalletInput {
            customer,
            asset_type,
        } = input;

        let user_id = user_id.0.ok_or(Error::new("X-USER-ID header not found"))?;
        let org_id = organization_id
            .0
            .ok_or(Error::new("X-ORGANIZATION-ID header not found"))?;
        let balance = balance
            .0
            .ok_or(Error::new("X-CREDIT-BALANCE header not found"))?;

        let (customer_treasury, treasury) = customer_treasuries::Entity::find()
            .join(
                JoinType::InnerJoin,
                customer_treasuries::Relation::Treasuries.def(),
            )
            .filter(customer_treasuries::Column::CustomerId.eq(customer))
            .select_also(treasuries::Entity)
            .one(conn)
            .await?
            .ok_or(Error::new("customer treasury not found"))?;

        let treasury = treasury.ok_or(Error::new("treasury not found"))?;

        let wallet = Wallets::find()
            .filter(
                wallets::Column::TreasuryId
                    .eq(treasury.id)
                    .and(wallets::Column::AssetId.eq(asset_type)),
            )
            .one(conn)
            .await?;

        if let Some(wallet) = wallet {
            if wallet.address.is_some() {
                return Err(Error::new(format!(
                    "wallet already exists for customer {customer} and asset type {asset_type} "
                )));
            }
        }

        let TransactionId(credits_deduction_id) = credits
            .submit_pending_deduction(
                org_id,
                user_id,
                Actions::CreateWallet,
                asset_type.into(),
                balance,
            )
            .await
            .map_err(|e| match e.kind() {
                DeductionErrorKind::InsufficientBalance { available, cost } => Error::new(format!(
                    "insufficient balance: available: {available}, cost: {cost}"
                )),
                DeductionErrorKind::MissingItem => Error::new("action not supported at this time"),
                DeductionErrorKind::InvalidCost(_) => Error::new("invalid cost"),
                DeductionErrorKind::Send(_) => {
                    Error::new("unable to send credit deduction request")
                },
            })?;

        let vault_asset = fireblocks
            .client()
            .create()
            .wallet(
                treasury.vault_id.clone(),
                fireblocks.assets().id(asset_type.as_str()),
                CreateVaultWallet {
                    eos_account_name: None,
                },
            )
            .await?;

        credits
            .confirm_deduction(TransactionId(credits_deduction_id))
            .await?;

        let wallet = wallets::ActiveModel {
            treasury_id: Set(treasury.id),
            address: Set(Some(vault_asset.address.clone())),
            created_at: Set(Utc::now().into()),
            removed_at: Set(None),
            created_by: Set(user_id),
            asset_id: Set(asset_type),
            deduction_id: Set(Some(credits_deduction_id)),
            ..Default::default()
        };
        let wallet = wallet.insert(conn).await?;

        let project_id = customer_treasury.project_id.to_string();

        let event = TreasuryEvents {
            event: Some(treasury_events::Event::CustomerWalletCreated(
                treasury_events::CustomerWallet {
                    project_id: project_id.clone(),
                    customer_id: customer_treasury.customer_id.to_string(),
                    blockchain: asset_type.into(),
                    wallet_address: vault_asset.address,
                },
            )),
        };
        let key = TreasuryEventKey {
            id: treasury.id.to_string(),
            user_id: user_id.to_string(),
            project_id,
        };

        producer.send(Some(&event), Some(&key)).await?;

        Ok(CreateCustomerWalletPayload { wallet })
    }
}

/// Input for creating a customer wallet.
#[derive(InputObject, Clone, Debug)]
pub struct CreateCustomerWalletInput {
    /// The customer ID.
    pub customer: Uuid,
    /// Blockchain for wallet creation.
    pub asset_type: wallets::AssetType,
}

/// Response after wallet creation.
#[derive(SimpleObject, Clone, Debug)]
pub struct CreateCustomerWalletPayload {
    // The wallet that was just created.
    pub wallet: wallets::Model,
}

impl From<wallets::AssetType> for treasury_events::Blockchain {
    fn from(value: wallets::AssetType) -> Self {
        match value {
            wallets::AssetType::Solana => treasury_events::Blockchain::Solana,
            wallets::AssetType::Matic => treasury_events::Blockchain::Polygon,
            wallets::AssetType::Eth => treasury_events::Blockchain::Ethereum,
        }
    }
}
