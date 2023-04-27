use fireblocks::objects::transaction::{
    CreateTransaction, ExtraParameters, RawMessageData, TransactionOperation, TransactionStatus,
    TransferPeerPath, UnsignedMessage,
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
        transactions, treasuries,
        wallets::{self, AssetType},
    },
    proto::{
        self,
        treasury_events::{DropCreated, DropMinted, DropUpdated, Event, MintTransfered},
        NftEventKey, Transaction, TransferMintTransaction, TreasuryEventKey, TreasuryEvents,
    },
};

/// This function emits a `DropCreated` event to a Producer of `TreasuryEvents`.
/// # Errors
///
/// This function will return an error if it fails to emit the event.
pub async fn emit_drop_created_event(
    producer: Producer<TreasuryEvents>,
    key: NftEventKey,
    payload: DropCreated,
) -> Result<()> {
    let event = TreasuryEvents {
        event: Some(Event::DropCreated(payload)),
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
    payload: DropMinted,
) -> Result<()> {
    let event = TreasuryEvents {
        event: Some(Event::DropMinted(payload)),
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

/// The function sends a raw transaction to Fireblocks and waits for it to be signed.
/// Once the transaction is signed, the function extracts the signature and indexes the transaction details in the database.
/// Returns a Result containing the transaction status and signature if successful, otherwise an error.
/// # Errors
/// This function fails if the Fireblocks API returns an error, or if the rpc returns an error or if it fails to index the transaction
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)]
pub async fn create_raw_transaction(
    k: NftEventKey,
    transaction: Transaction,
    project_id: String,
    vault: String,
    conn: Connection,
    fireblocks: fireblocks::Client,
    rpc: &RpcClient,
    t: TxType,
) -> Result<(TransactionStatus, Signature)> {
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

    let tx_type: String = t.clone().into();

    let note = Some(format!(
        "{:?} by {:?} for project {:?}",
        tx_type, k.user_id, project_id
    ));

    let blockchain: proto::Blockchain =
        proto::Blockchain::from_i32(blockchain).context("can not parse blockchain enum")?;
    let asset_ids: Vec<AssetType> = blockchain.try_into()?;

    let wallet = wallets::Entity::find()
        .join(JoinType::InnerJoin, wallets::Relation::Treasuries.def())
        .filter(treasuries::Column::VaultId.eq(vault.clone()))
        .filter(wallets::Column::AssetId.is_in(asset_ids))
        .one(conn.get())
        .await?
        .context("wallet not found")?;

    let tx = CreateTransaction {
        asset_id: wallet.asset_id.into(),
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

    let rpc_response = rpc.send_transaction(&signed_transaction);
    info!("RPC response {:?}", rpc_response);

    let signature = rpc_response?;
    info!("{:?} signature {:?}", note, signature);

    index_transaction(conn.get(), tx_details.id, signature, t).await?;

    Ok((tx_details.status, signature))
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

    let vault = treasuries::Entity::find()
        .join(
            JoinType::InnerJoin,
            treasuries::Relation::ProjectTreasury.def(),
        )
        .filter(project_treasuries::Column::ProjectId.eq(project))
        .one(db)
        .await?
        .context("treasury not found in database")?
        .vault_id;

    Ok(vault)
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
