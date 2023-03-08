use fireblocks::objects::{
    transaction::{
        CreateTransaction, ExtraParameters, RawMessageData, TransactionOperation,
        TransactionStatus, TransferPeerPath, UnsignedMessage,
    },
    vault::{CreateVault, CreateVaultWallet},
};
use hex::FromHex;
use hub_core::{prelude::*, producer::Producer, tokio::time, uuid::Uuid};
use sea_orm::{prelude::*, JoinType, QuerySelect, Set};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{signature::Signature, transaction::Transaction as SplTransaction};

use crate::{
    db::Connection,
    entities::{
        customer_treasuries, project_treasuries, treasuries,
        wallets::{self, AssetType},
    },
    proto::{
        customer_events, nft_events,
        organization_events::{self},
        treasury_events::{
            CustomerTreasury, DropCreated, DropMinted, ProjectWallet, {self},
        },
        Customer, CustomerEventKey, NftEventKey, OrganizationEventKey, Project, Transaction,
        TreasuryEventKey, TreasuryEvents,
    },
    Services,
};

/// Res
///
/// # Errors
/// This function fails if ...
pub async fn process(
    msg: Services,
    db: Connection,
    fireblocks: fireblocks::Client,
    supported_ids: Vec<String>,
    rpc: &RpcClient,
    producer: Producer<TreasuryEvents>,
) -> Result<()> {
    // match topics
    match msg {
        Services::Customers(key, e) => match e.event {
            Some(customer_events::Event::Created(customer)) => {
                create_customer_treasury(db, fireblocks, producer, key, customer).await
            },
            None => Ok(()),
        },
        Services::Organizations(key, e) => match e.event {
            Some(organization_events::Event::ProjectCreated(p)) => {
                create_project_treasury(key, p, db, fireblocks, producer, supported_ids).await
            },
            Some(_) | None => Ok(()),
        },
        Services::Nfts(key, e) => match e.event {
            // match topic messages
            Some(nft_events::Event::CreateDrop(payload)) => {
                let status = create_raw_transaction(
                    key.clone(),
                    payload.transaction.context("transaction not found")?,
                    payload.project_id.clone(),
                    db,
                    fireblocks,
                    rpc,
                    Transactions::CreateMasterEdition,
                )
                .await?;

                emit_drop_created_event(producer, key.id, DropCreated {
                    project_id: payload.project_id,
                    status: status as i32,
                })
                .await
                .context("failed to emit drop_created event")?;

                Ok(())
            },
            Some(nft_events::Event::MintDrop(payload)) => {
                let status = create_raw_transaction(
                    key.clone(),
                    payload.transaction.context("transaction not found")?,
                    payload.project_id.clone(),
                    db,
                    fireblocks,
                    rpc,
                    Transactions::MintEdition,
                )
                .await?;

                emit_drop_minted_event(producer, key.id, DropMinted {
                    project_id: payload.project_id,
                    drop_id: payload.drop_id,
                    status: status as i32,
                })
                .await
                .context("failed to emit drop_created event")?;

                Ok(())
            },
            None => Ok(()),
        },
    }
}

/// Res
///
/// # Errors
/// This function fails if ...
pub async fn create_project_treasury(
    k: OrganizationEventKey,
    project: Project,
    conn: Connection,
    fireblocks: fireblocks::Client,
    producer: Producer<TreasuryEvents>,
    supported_ids: Vec<String>,
) -> Result<()> {
    let user_id = Uuid::from_str(&k.user_id)?;

    let asset_types: Vec<AssetType> = supported_ids
        .iter()
        .map(|a| AssetType::from_str(a))
        .into_iter()
        .collect::<Result<Vec<AssetType>>>()?;

    let create_vault = CreateVault {
        name: format!("project:{}", project.id.clone()),
        hidden_on_ui: None,
        customer_ref_id: Some(k.user_id),
        auto_fuel: Some(false),
    };

    let vault = fireblocks.create_vault(create_vault).await?;

    let treasury = treasuries::ActiveModel {
        vault_id: Set(vault.id.clone()),
        ..Default::default()
    };

    let treasury: treasuries::Model = treasury
        .clone()
        .insert(conn.get())
        .await
        .context("failed to get treasury record from db")?;

    let project_treasuries_active_model = project_treasuries::ActiveModel {
        project_id: Set(Uuid::parse_str(&project.id).context("failed to parse project id to Uuid")?),
        treasury_id: Set(treasury.id),
        ..Default::default()
    };

    project_treasuries_active_model
        .insert(conn.get())
        .await
        .context("failed to insert project treasuries")?;

    info!("treasury created for project {:?}", project.id);

    // create vault wallets for supported assets
    for asset_type in asset_types {
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

        active_model.insert(conn.get()).await?;

        let event = treasury_events::Event::ProjectWalletCreated(ProjectWallet {
            project_id: project.id.to_string(),
            wallet_address: vault_asset.address,
            blockchain: asset_type.into(),
        });

        let event = TreasuryEvents { event: Some(event) };
        let key = TreasuryEventKey {
            id: treasury.id.to_string(),
        };

        producer.send(Some(&event), Some(&key)).await?;
    }

    Ok(())
}

/// Res
///
/// # Errors
/// This function fails if ...
pub async fn create_customer_treasury(
    conn: Connection,
    fireblocks: fireblocks::Client,
    producer: Producer<TreasuryEvents>,
    key: CustomerEventKey,
    customer: Customer,
) -> Result<()> {
    let create_vault = CreateVault {
        name: format!("customer:{}", key.id.clone()),
        hidden_on_ui: None,
        customer_ref_id: None,
        auto_fuel: Some(false),
    };

    let vault = fireblocks.create_vault(create_vault).await?;

    info!("vault created {:?}", vault);

    let treasury = treasuries::ActiveModel {
        vault_id: Set(vault.id.clone()),
        ..Default::default()
    };

    let treasury: treasuries::Model = treasury
        .clone()
        .insert(conn.get())
        .await
        .context("failed to insert treasury record")?;

    let project_id = Uuid::from_str(&customer.project_id)?;

    let customer_am = customer_treasuries::ActiveModel {
        customer_id: Set(Uuid::parse_str(&key.id).context("failed to parse customer id to Uuid")?),
        treasury_id: Set(treasury.id),
        project_id: Set(project_id),
        ..Default::default()
    };

    customer_am
        .insert(conn.get())
        .await
        .context("failed to insert customer treasuries")?;

    info!("treasury created for customer {:?}", key.id);

    let event = TreasuryEvents {
        event: Some(treasury_events::Event::CustomerTreasuryCreated(
            CustomerTreasury {
                customer_id: key.id,
                project_id: customer.project_id,
            },
        )),
    };

    let key = TreasuryEventKey {
        id: treasury.id.to_string(),
    };

    producer.send(Some(&event), Some(&key)).await?;

    Ok(())
}

/// Res
///
/// # Errors
/// This function fails if ...
pub async fn create_raw_transaction(
    k: NftEventKey,
    transaction: Transaction,
    project_id: String,
    conn: Connection,
    fireblocks: fireblocks::Client,
    rpc: &RpcClient,
    t: Transactions,
) -> Result<TransactionStatus> {
    let Transaction {
        serialized_message,
        signed_message_signatures,
    } = transaction;

    let project = Uuid::parse_str(&project_id)?;
    let payer_signature = signed_message_signatures
        .get(0)
        .context("failed to get payer signature")?;
    let mint_signature = signed_message_signatures
        .get(1)
        .context("failed to get mint signature")?;
    let note = Some(format!(
        "{:?} by {:?} for project {:?}",
        t.to_string(),
        k.user_id,
        project_id
    ));

    let vault = treasuries::Entity::find()
        .join(
            JoinType::InnerJoin,
            treasuries::Relation::ProjectTreasury.def(),
        )
        .filter(project_treasuries::Column::ProjectId.eq(project))
        .one(conn.get())
        .await?
        .context("treasury not found in database")?
        .vault_id;

    let tx = CreateTransaction {
        asset_id: "SOL_TEST".to_string(),
        operation: TransactionOperation::RAW,
        source: TransferPeerPath {
            peer_type: "VAULT_ACCOUNT".to_string(),
            id: vault,
        },
        destination: None,
        destinations: None,
        treat_as_gross_amount: None,
        customer_ref_id: None,
        amount: "0".to_string(),
        extra_parameters: Some(ExtraParameters::RawMessageData(RawMessageData {
            messages: vec![UnsignedMessage {
                content: hex::encode(&serialized_message),
            }],
        })),
        note: note.clone(),
    };

    let mut interval = time::interval(time::Duration::from_millis(250));

    let transaction = fireblocks.create_transaction(tx).await?;

    let tx_details = loop {
        let tx_details = fireblocks.get_transaction(transaction.id.clone()).await?;

        if tx_details.clone().signed_messages.is_empty() {
            interval.tick().await;

            continue;
        }

        break tx_details;
    };

    let full_sig = tx_details
        .signed_messages
        .get(0)
        .context("failed to get signed message response")?
        .clone()
        .signature
        .full_sig;

    // SIGNATURE LEN = 64 bytes

    let signature_decoded = <[u8; 64]>::from_hex(full_sig)?;

    let signature = Signature::new(&signature_decoded);

    let decoded_message = bincode::deserialize(&serialized_message)?;

    let signed_transaction = SplTransaction {
        signatures: vec![
            Signature::from_str(payer_signature)?,
            Signature::from_str(mint_signature)?,
            signature,
        ],
        message: decoded_message,
    };

    let res = rpc.send_transaction(&signed_transaction)?;

    info!("{:?} signature {:?}", note, res);

    Ok(tx_details.status)

    // Ok(())
}

async fn emit_drop_created_event(
    producer: Producer<TreasuryEvents>,
    id: String,
    payload: DropCreated,
) -> Result<()> {
    let event = TreasuryEvents {
        event: Some(treasury_events::Event::DropCreated(payload)),
    };

    let key = TreasuryEventKey { id };

    producer
        .send(Some(&event), Some(&key))
        .await
        .map_err(Into::into)
}

async fn emit_drop_minted_event(
    producer: Producer<TreasuryEvents>,
    id: String,
    payload: DropMinted,
) -> Result<()> {
    let event = TreasuryEvents {
        event: Some(treasury_events::Event::DropMinted(payload)),
    };

    let key = TreasuryEventKey { id };

    producer
        .send(Some(&event), Some(&key))
        .await
        .map_err(Into::into)
}

#[derive(Debug)]
pub enum Transactions {
    CreateMasterEdition,
    MintEdition,
}

impl fmt::Display for Transactions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
