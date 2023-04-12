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
        customer_treasuries, project_treasuries,
        sea_orm_active_enums::TxType,
        transactions, treasuries,
        wallets::{self, AssetType},
    },
    proto::{
        customer_events, nft_events,
        organization_events::{self},
        treasury_events::{
            CustomerTreasury, DropCreated, DropMinted, DropUpdated, ProjectWallet, {self},
        },
        Blockchain, Customer, CustomerEventKey, NftEventKey, OrganizationEventKey, Project,
        Transaction, TreasuryEventKey, TreasuryEvents,
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
                let (status, sig) = create_raw_transaction(
                    key.clone(),
                    payload.transaction.context("transaction not found")?,
                    payload.project_id.clone(),
                    db,
                    fireblocks,
                    rpc,
                    TxType::CreateDrop,
                )
                .await?;

                emit_drop_created_event(producer, key, DropCreated {
                    project_id: payload.project_id,
                    status: status as i32,
                    tx_signature: sig.to_string(),
                })
                .await
                .context("failed to emit drop_created event")?;

                Ok(())
            },
            Some(nft_events::Event::MintDrop(payload)) => {
                let (status, sig) = create_raw_transaction(
                    key.clone(),
                    payload.transaction.context("transaction not found")?,
                    payload.project_id.clone(),
                    db,
                    fireblocks,
                    rpc,
                    TxType::MintEdition,
                )
                .await?;

                emit_drop_minted_event(producer, key, DropMinted {
                    project_id: payload.project_id,
                    drop_id: payload.drop_id,
                    status: status as i32,
                    tx_signature: sig.to_string(),
                })
                .await
                .context("failed to emit drop_created event")?;

                Ok(())
            },
            Some(nft_events::Event::UpdateMetadata(payload)) => {
                let (status, sig) = create_raw_transaction(
                    key.clone(),
                    payload.transaction.context("transaction not found")?,
                    payload.project_id.clone(),
                    db,
                    fireblocks,
                    rpc,
                    TxType::UpdateMetadata,
                )
                .await?;

                emit_drop_updated_event(producer, key, DropUpdated {
                    project_id: payload.project_id,
                    drop_id: payload.drop_id,
                    status: status as i32,
                    tx_signature: sig.to_string(),
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

        let proto_blockchain_enum: Blockchain = asset_type.into();

        let event = treasury_events::Event::ProjectWalletCreated(ProjectWallet {
            project_id: project.id.to_string(),
            wallet_address: vault_asset.address,
            blockchain: proto_blockchain_enum as i32,
        });

        let event = TreasuryEvents { event: Some(event) };
        let key = TreasuryEventKey {
            id: treasury.id.to_string(),
            user_id: user_id.to_string(),
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
                customer_id: key.id.clone(),
                project_id: customer.project_id,
            },
        )),
    };

    let key = TreasuryEventKey {
        id: treasury.id.to_string(),
        user_id: key.id,
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
    t: TxType,
) -> Result<(TransactionStatus, Signature)> {
    let Transaction {
        serialized_message,
        signed_message_signatures,
        ..
    } = transaction;

    let project = Uuid::parse_str(&project_id)?;

    let mut signed_signatures = signed_message_signatures
        .iter()
        .map(|s| {
            Signature::from_str(s).map_err(|e| anyhow!(format!("failed to parse signature: {e}")))
        })
        .collect::<Result<Vec<Signature>>>()?;

    let tx_type: String = t.clone().into();

    let note = Some(format!(
        "{:?} by {:?} for project {:?}",
        tx_type, k.user_id, project_id
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
        asset_id: "SOL".to_string(),
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

        match tx_details.clone().status {
            TransactionStatus::FAILED
            | TransactionStatus::COMPLETED
            | TransactionStatus::BLOCKED
            | TransactionStatus::CANCELLED
            | TransactionStatus::REJECTED => {
                break tx_details;
            },
            _ => {
                interval.tick().await;

                continue;
            },
        }
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
    signed_signatures.push(Signature::new(&signature_decoded));

    let decoded_message = bincode::deserialize(&serialized_message)?;

    let signed_transaction = SplTransaction {
        signatures: signed_signatures,
        message: decoded_message,
    };

    let sig = rpc.send_transaction(&signed_transaction)?;

    info!("{:?} signature {:?}", note, sig);

    index_transaction(conn.get(), tx_details.id, sig, t).await?;

    Ok((tx_details.status, sig))
}

async fn index_transaction(
    db: &DatabaseConnection,
    id: String,
    signature: Signature,
    t: TxType,
) -> Result<()> {
    let tx_am = transactions::ActiveModel {
        fireblocks_id: Set(Uuid::from_str(&id)?),
        signature: Set(signature.to_string()),
        tx_type: Set(t),
        ..Default::default()
    };

    tx_am.insert(db).await?;

    Ok(())
}

async fn emit_drop_created_event(
    producer: Producer<TreasuryEvents>,
    key: NftEventKey,
    payload: DropCreated,
) -> Result<()> {
    let event = TreasuryEvents {
        event: Some(treasury_events::Event::DropCreated(payload)),
    };

    let key = TreasuryEventKey {
        id: key.id,
        user_id: key.user_id,
    };

    producer
        .send(Some(&event), Some(&key))
        .await
        .map_err(Into::into)
}

async fn emit_drop_minted_event(
    producer: Producer<TreasuryEvents>,
    key: NftEventKey,
    payload: DropMinted,
) -> Result<()> {
    let event = TreasuryEvents {
        event: Some(treasury_events::Event::DropMinted(payload)),
    };

    let key = TreasuryEventKey {
        id: key.id,
        user_id: key.user_id,
    };

    producer
        .send(Some(&event), Some(&key))
        .await
        .map_err(Into::into)
}

async fn emit_drop_updated_event(
    producer: Producer<TreasuryEvents>,
    key: NftEventKey,
    payload: DropUpdated,
) -> Result<()> {
    let event = TreasuryEvents {
        event: Some(treasury_events::Event::DropUpdated(payload)),
    };

    let key = TreasuryEventKey {
        id: key.id,
        user_id: key.user_id,
    };

    producer
        .send(Some(&event), Some(&key))
        .await
        .map_err(Into::into)
}

impl From<AssetType> for Blockchain {
    fn from(value: AssetType) -> Self {
        match value {
            AssetType::Solana => Blockchain::Solana,
            AssetType::SolanaTest => Blockchain::Solana,
            AssetType::MaticTest => Blockchain::Polygon,
            AssetType::Matic => Blockchain::Polygon,
            AssetType::EthTest => Blockchain::Ethereum,
            AssetType::Eth => Blockchain::Ethereum,
        }
    }
}
