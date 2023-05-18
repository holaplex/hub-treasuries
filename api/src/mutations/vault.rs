use async_graphql::{Context, Error, InputObject, Object, Result, SimpleObject};
use fireblocks::{objects::vault::CreateVaultWallet, Client as FireblocksClient};
use hub_core::{
    chrono::Utc,
    credits::{CreditsClient, TransactionId},
    producer::Producer,
};
use sea_orm::{prelude::*, JoinType, QuerySelect, Set};

use crate::{
    db::Connection,
    entities::{
        customer_treasuries,
        prelude::Wallets,
        treasuries,
        wallets::{self, AssetType},
    },
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
        let fireblocks = ctx.data::<FireblocksClient>()?;
        let credits = ctx.data::<CreditsClient<Actions>>()?;
        let conn = db.get();
        let producer = ctx.data::<Producer<TreasuryEvents>>()?;
        let CreateCustomerWalletInput {
            customer,
            asset_type,
        } = input;

        let user_id = user_id
            .0
            .ok_or_else(|| Error::new("X-USER-ID header not found"))?;
        let org_id = organization_id
            .0
            .ok_or_else(|| Error::new("X-ORGANIZATION-ID header not found"))?;
        let balance = balance
            .0
            .ok_or_else(|| Error::new("X-CREDIT-BALANCE header not found"))?;

        let (customer_treasury, treasury) = customer_treasuries::Entity::find()
            .join(
                JoinType::InnerJoin,
                customer_treasuries::Relation::Treasuries.def(),
            )
            .filter(customer_treasuries::Column::CustomerId.eq(customer))
            .select_also(treasuries::Entity)
            .one(conn)
            .await?
            .ok_or_else(|| Error::new("customer treasury not found"))?;

        let treasury = treasury.ok_or_else(|| Error::new("treasury not found"))?;

        let deduction_id = submit_pending_deduction(credits, db, DeductionParams {
            balance,
            user_id,
            customer,
            org_id,
            treasury: treasury.id,
            asset_type,
        })
        .await?;

        let vault_asset = fireblocks
            .create_vault_wallet(
                treasury.vault_id.clone(),
                asset_type.into(),
                CreateVaultWallet {
                    eos_account_name: None,
                },
            )
            .await?;

        credits
            .confirm_deduction(TransactionId(deduction_id))
            .await?;

        let wallet = update_wallet_address(db, vault_asset.address, deduction_id).await?;

        let event = TreasuryEvents {
            event: Some(treasury_events::Event::CustomerWalletCreated(
                treasury_events::CustomerWallet {
                    project_id: customer_treasury.project_id.to_string(),
                    customer_id: customer_treasury.customer_id.to_string(),
                    blockchain: asset_type.into(),
                },
            )),
        };
        let key = TreasuryEventKey {
            id: treasury.id.to_string(),
            user_id: user_id.to_string(),
        };

        producer.send(Some(&event), Some(&key)).await?;

        Ok(CreateCustomerWalletPayload { wallet })
    }
}

/// Checks if the wallet already exists for the given treasury and asset type.
/// Returns the ID of an existing wallet  entry if it exists for the given treasury and asset type.
/// Otherwise, it generates a new pending deduction ID using the `CreditsClient`
/// and creates a new entry in the wallets table, returning its ID.
/// #Errors
/// May return an error if there is an issue with querying or inserting data, or if the asset type is not supported.
async fn submit_pending_deduction(
    credits: &CreditsClient<Actions>,
    db: &Connection,
    params: DeductionParams,
) -> Result<Uuid> {
    let DeductionParams {
        balance,
        user_id,
        customer,
        org_id,
        treasury,
        asset_type,
    } = params;

    let wallet = Wallets::find()
        .filter(
            wallets::Column::TreasuryId
                .eq(treasury)
                .and(wallets::Column::AssetId.eq(asset_type)),
        )
        .one(db.get())
        .await?;

    if let Some(wallet) = wallet {
        match (wallet.address, wallet.deduction_id) {
            (Some(_), _) => {
                return Err(Error::new(format!(
                    "wallet already exists for customer {customer} and asset type {asset_type} "
                )));
            },
            (_, Some(deduction_id)) => {
                return Ok(deduction_id);
            },
            _ => {},
        }
    }

    let id = match asset_type {
        AssetType::Solana | AssetType::SolanaTest => {
            credits
                .submit_pending_deduction(
                    org_id,
                    user_id,
                    Actions::CreateWallet,
                    hub_core::credits::Blockchain::Solana,
                    balance,
                )
                .await?
        },
        _ => {
            return Err(Error::new("blockchain not supported yet"));
        },
    };

    let deduction_id = id
        .ok_or_else(|| Error::new("failed to generate credits deduction id"))?
        .0;

    let wallet_model = wallets::ActiveModel {
        treasury_id: Set(treasury),
        address: Set(None),
        created_at: Set(Utc::now().into()),
        removed_at: Set(None),
        created_by: Set(user_id),
        asset_id: Set(asset_type),
        deduction_id: Set(Some(deduction_id)),
        ..Default::default()
    };
    wallet_model.insert(db.get()).await?;

    Ok(deduction_id)
}

/// Updates the address of the wallet record with the specified UUID
/// # Errors
/// Fails if the wallet is not found or fails to update the record.
async fn update_wallet_address(
    db: &Connection,
    address: String,
    deduction_id: Uuid,
) -> Result<wallets::Model> {
    let wallet_model = Wallets::find()
        .filter(wallets::Column::DeductionId.eq(deduction_id))
        .one(db.get())
        .await?
        .ok_or_else(|| Error::new("wallet not found"))?;

    let mut wallet_am: wallets::ActiveModel = wallet_model.into();
    wallet_am.address = Set(Some(address));
    let wallet = wallet_am.update(db.get()).await?;

    Ok(wallet)
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

struct DeductionParams {
    balance: u64,
    user_id: Uuid,
    customer: Uuid,
    org_id: Uuid,
    treasury: Uuid,
    asset_type: AssetType,
}
