use fireblocks::objects::transaction::{
    CreateTransaction, ExtraParameters, RawMessageData, TransactionDetails, TransactionOperation,
    TransactionStatus, TransferPeerPath, UnsignedMessage,
};
use hex::FromHex;
use hub_core::{prelude::*, producer::Producer, tokio::time, uuid::Uuid};
use sea_orm::{prelude::*, JoinType, QuerySelect, Set};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{signature::Signature, transaction::Transaction as SplTransaction};

use crate::{
    db::Connection,
    entities::{
        prelude::{Treasuries, Wallets},
        project_treasuries,
        sea_orm_active_enums::TxType,
        transactions, treasuries, wallets,
    },
    proto::{
        self,
        treasury_events::{DropCreated, DropMinted, DropUpdated, Event, MintTransfered},
        NftEventKey, Transaction, TransferMintTransaction, TreasuryEventKey, TreasuryEvents,
    },
    BLOCKCHAIN_ASSET_IDS,
};

/// This function emits a `DropCreated` event to a Producer of `TreasuryEvents`.
/// # Errors
///
/// This function will return an error if it fails to emit the event.
pub async fn emit_drop_created_event(
    producer: Producer<TreasuryEvents>,
    key: NftEventKey,
    project_id: String,
    status: TransactionStatus,
    tx_signature: String,
) -> Result<()> {
    let event = TreasuryEvents {
        event: Some(Event::DropCreated(DropCreated {
            project_id,
            status: status as i32,
            tx_signature,
        })),
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

/// This function emits a `DropCreated` event to a Producer of `TreasuryEvents`.
/// # Errors
///
/// This function will return an error if it fails to emit the event.
pub async fn emit_drop_retried_event(
    producer: Producer<TreasuryEvents>,
    key: NftEventKey,
    project_id: String,
    status: TransactionStatus,
    signature: String,
) -> Result<()> {
    let event = TreasuryEvents {
        event: Some(Event::DropRetried(DropCreated {
            project_id,
            status: status as i32,
            tx_signature: signature,
        })),
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

/// This function emits a `DropMinted` event.
/// # Errors
///
/// This function will return an error if it fails to emit the event.
pub async fn emit_drop_minted_event(
    producer: Producer<TreasuryEvents>,
    key: NftEventKey,
    project_id: String,
    drop_id: String,
    status: TransactionStatus,
    signature: String,
) -> Result<()> {
    let event = TreasuryEvents {
        event: Some(Event::DropMinted(DropMinted {
            project_id,
            drop_id,
            status: status as i32,
            tx_signature: signature,
        })),
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

/// This function emits a `DropMinted` event.
/// # Errors
///
/// This function will return an error if it fails to emit the event.
pub async fn emit_mint_retried_event(
    producer: Producer<TreasuryEvents>,
    key: NftEventKey,
    project_id: String,
    drop_id: String,
    status: TransactionStatus,
    signature: String,
) -> Result<()> {
    let event = TreasuryEvents {
        event: Some(Event::MintRetried(DropMinted {
            project_id,
            drop_id,
            status: status as i32,
            tx_signature: signature,
        })),
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

/// This function emits a `DropUpdated` event.
/// # Errors
///
/// This function will return an error if it fails to emit the event.
pub async fn emit_drop_updated_event(
    producer: Producer<TreasuryEvents>,
    key: NftEventKey,
    payload: DropUpdated,
) -> Result<()> {
    let event = TreasuryEvents {
        event: Some(Event::DropUpdated(payload)),
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

/// This function emits a `MintTransfered` event.
/// # Errors
///
/// This function will return an error if it fails to emit the event.
pub async fn emit_mint_transfered_event(
    producer: Producer<TreasuryEvents>,
    key: NftEventKey,
    payload: TransferMintTransaction,
    tx_signature: String,
) -> Result<()> {
    let event = TreasuryEvents {
        event: Some(Event::MintTransfered(MintTransfered {
            sender: payload.sender,
            recipient: payload.recipient,
            mint_address: payload.address,
            tx_signature,
            project_id: payload.project_id,
            transfer_id: payload.transfer_id,
        })),
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

pub struct RawTxParams {
    pub key: NftEventKey,
    pub transaction: Transaction,
    pub project_id: String,
    pub vault: String,
    pub tx_type: TxType,
    pub treasury_vault_id: String,
}

/// The function sends a raw transaction to Fireblocks and waits for it to be signed.
/// Once the transaction is signed, the function extracts the signature and indexes the transaction details in the database.
/// Returns a Result containing the transaction status and signature if successful, otherwise an error.
/// # Errors
/// This function fails if the Fireblocks API returns an error, or if the rpc returns an error or if it fails to index the transaction
pub async fn create_raw_transaction(
    conn: Connection,
    fireblocks: fireblocks::Client,
    rpc: &RpcClient,
    params: RawTxParams,
) -> Result<(TransactionStatus, Signature)> {
    let RawTxParams {
        key,
        transaction,
        project_id,
        vault,
        tx_type,
        treasury_vault_id,
    } = params;

    let Transaction {
        serialized_message,
        signed_message_signatures,
        blockchain,
    } = transaction;

    let mut signed_signatures = signed_message_signatures
        .iter()
        .map(|s| {
            Signature::from_str(s).map_err(|e| anyhow!(format!("failed to parse signature: {e}")))
        })
        .collect::<Result<Vec<Signature>>>()?;

    let note = Some(format!(
        "{:?} by {:?} for project {:?}",
        tx_type, key.user_id, project_id
    ));

    let blockchain: proto::Blockchain =
        proto::Blockchain::from_i32(blockchain).context("can not parse blockchain enum")?;

    let asset_id = (*BLOCKCHAIN_ASSET_IDS
        .get()
        .context("failed to get blockchain asset ids")?
        .get(&blockchain)
        .ok_or_else(|| anyhow!("asset id not found for blockchain {:?}", blockchain))?)
    .to_string();

    let (_, payer_signature) = create_transaction(
        fireblocks.clone(),
        asset_id.clone(),
        treasury_vault_id,
        &serialized_message,
        &note,
    )
    .await?;

    let (tx_details, project_treasury_signature) = create_transaction(
        fireblocks.clone(),
        asset_id,
        vault,
        &serialized_message,
        &note,
    )
    .await?;

    signed_signatures.extend([payer_signature, project_treasury_signature]);

    let decoded_message = bincode::deserialize(&serialized_message)?;

    let signed_transaction = SplTransaction {
        signatures: signed_signatures,
        message: decoded_message,
    };

    let rpc_response = rpc.send_transaction(&signed_transaction);
    info!("RPC response {:?}", rpc_response);

    let signature = rpc_response?;
    info!("{:?} signature {:?}", note, signature);

    index_transaction(conn.get(), tx_details.id, signature, tx_type).await?;

    Ok((tx_details.status, signature))
}

async fn create_transaction(
    fireblocks: fireblocks::Client,
    asset_id: String,
    vault: String,
    serialized_message: &Vec<u8>,
    note: &Option<String>,
) -> Result<(TransactionDetails, Signature)> {
    let tx = CreateTransaction {
        asset_id: asset_id.to_string(),
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
                content: hex::encode(serialized_message),
            }],
        })),
        note: note.clone(),
    };

    let mut interval = time::interval(time::Duration::from_millis(250));

    let transaction = fireblocks.create_transaction(tx).await?;

    let tx_details = loop {
        let tx_details = fireblocks.get_transaction(transaction.id.clone()).await?;

        match tx_details.clone().status {
            TransactionStatus::SUBMITTED
            | TransactionStatus::QUEUED
            | TransactionStatus::BROADCASTING
            | TransactionStatus::CONFIRMING => {
                interval.tick().await;

                continue;
            },
            TransactionStatus::COMPLETED => {
                break tx_details;
            },
            _ => return Ok((tx_details, Signature::default())),
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

    Ok((tx_details, Signature::new(&signature_decoded)))
}

/// This is a helper function used by `create_raw_transaction` to index the transaction details in the database.
/// It inserts the transaction details, including the Fireblocks ID, signature, and transaction type
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

/// This function finds the vault ID associated with a project ID in the database.
/// It queries the `treasuries` table and retrieves the vault ID based on the given project ID.
pub(crate) async fn find_vault_id_by_project_id(
    db: &DatabaseConnection,
    project: String,
) -> Result<String> {
    let project = Uuid::from_str(&project)?;

    let (_, t) = project_treasuries::Entity::find()
        .find_also_related(treasuries::Entity)
        .filter(project_treasuries::Column::ProjectId.eq(project))
        .one(db)
        .await?
        .context("treasury not found in database")?;

    let t = t.ok_or_else(|| anyhow!("treasury not found"))?;

    Ok(t.vault_id)
}

/// This function finds the vault ID associated with a wallet address in the database.
/// It queries the `treasuries` and `wallets` tables and retrieves the vault ID based on the given wallet address.
pub(crate) async fn find_vault_id_by_wallet_address(
    db: &DatabaseConnection,
    address: String,
) -> Result<String> {
    let (treasury, _) = Treasuries::find()
        .find_also_related(Wallets)
        .filter(wallets::Column::Address.eq(address))
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("wallet not found"))?;

    Ok(treasury.vault_id)
}
