pub mod customer;
pub mod nft;
pub mod organization;
use customer::create_customer_treasury;
use fireblocks::objects::transaction::TransactionStatus;
use hub_core::{prelude::*, producer::Producer};
use nft::{
    create_raw_transaction, emit_drop_created_event, emit_drop_minted_event,
    emit_drop_updated_event,
};
use organization::create_project_treasury;
use solana_client::rpc_client::RpcClient;

use self::nft::{
    emit_mint_transfered_event, find_vault_id_by_project_id, find_vault_id_by_wallet_address,
    RawTxParams,
};
use crate::{
    db::Connection,
    entities::sea_orm_active_enums::TxType,
    events::nft::{emit_drop_retried_event, emit_mint_retried_event},
    proto::{
        customer_events::Event as CustomerEvent, nft_events::Event as NftEvent,
        organization_events::Event as OrganizationEvent, treasury_events::DropUpdated,
        TreasuryEvents,
    },
    Services,
};

/// This function processes different types of events related to various services.
///
/// # Arguments
/// * `msg`: A message indicating the service and event to be processed.
/// * `db`: A connection to the database.
/// * `fireblocks`: A client for interacting with the Fireblocks API.
/// * `supported_ids`: A vector of supported IDs.
/// * `rpc`: A reference to the `RpcClient`.
/// * `producer`: A producer for sending events to a message broker.
///
/// # Errors
/// This function may return an error in the following cases:
/// * Fails to process the event due to any reason such as failure in interacting with the database, Fireblocks API, or `RpcClient`.
/// * Fails to emit events to the message broker.
#[allow(clippy::too_many_lines)]
pub async fn process(
    msg: Services,
    db: Connection,
    fireblocks: fireblocks::Client,
    supported_ids: Vec<String>,
    treasury_vault_id: String,
    rpc: &RpcClient,
    producer: Producer<TreasuryEvents>,
) -> Result<()> {
    // match topics
    match msg {
        Services::Customers(key, e) => match e.event {
            Some(CustomerEvent::Created(customer)) => {
                create_customer_treasury(db, fireblocks, producer, key, customer).await
            },
            Some(_) | None => Ok(()),
        },
        Services::Organizations(key, e) => match e.event {
            Some(OrganizationEvent::ProjectCreated(p)) => {
                create_project_treasury(key, p, db, fireblocks, producer, supported_ids).await
            },
            Some(_) | None => Ok(()),
        },
        Services::Nfts(key, e) => match e.event {
            // match topic messages
            Some(NftEvent::CreateDrop(payload)) => {
                let vault =
                    find_vault_id_by_project_id(db.get(), payload.project_id.clone()).await?;

                let (status, signature) =
                    match create_raw_transaction(db, fireblocks, rpc, RawTxParams {
                        key: key.clone(),
                        transaction: payload.transaction.context("transaction not found")?,
                        project_id: payload.project_id.clone(),
                        vault: vault.to_string(),
                        tx_type: TxType::CreateDrop,
                        treasury_vault_id,
                    })
                    .await
                    {
                        Ok((status, signature)) => (status, signature.to_string()),
                        Err(_) => (TransactionStatus::FAILED, String::new()),
                    };

                emit_drop_created_event(producer, key, payload.project_id, status, signature)
                    .await
                    .context("failed to emit drop_created event")?;

                Ok(())
            },
            Some(NftEvent::RetryDrop(payload)) => {
                let vault =
                    find_vault_id_by_project_id(db.get(), payload.project_id.clone()).await?;

                let (status, signature) =
                    match create_raw_transaction(db, fireblocks, rpc, RawTxParams {
                        key: key.clone(),
                        transaction: payload.transaction.context("transaction not found")?,
                        project_id: payload.project_id.clone(),
                        vault: vault.to_string(),
                        tx_type: TxType::CreateDrop,
                        treasury_vault_id,
                    })
                    .await
                    {
                        Ok((status, signature)) => (status, signature.to_string()),
                        Err(_) => (TransactionStatus::FAILED, String::new()),
                    };

                emit_drop_retried_event(producer, key, payload.project_id, status, signature)
                    .await
                    .context("failed to emit drop retried event")?;

                Ok(())
            },
            Some(NftEvent::MintDrop(payload)) => {
                let vault =
                    find_vault_id_by_project_id(db.get(), payload.project_id.clone()).await?;

                let (status, signature) =
                    match create_raw_transaction(db, fireblocks, rpc, RawTxParams {
                        key: key.clone(),
                        transaction: payload.transaction.context("transaction not found")?,
                        project_id: payload.project_id.clone(),
                        vault,
                        tx_type: TxType::MintEdition,
                        treasury_vault_id,
                    })
                    .await
                    {
                        Ok((status, signature)) => (status, signature.to_string()),
                        Err(_) => (TransactionStatus::FAILED, String::new()),
                    };

                emit_drop_minted_event(
                    producer,
                    key,
                    payload.project_id,
                    payload.drop_id,
                    status,
                    signature,
                )
                .await?;

                Ok(())
            },
            Some(NftEvent::RetryMint(payload)) => {
                let vault =
                    find_vault_id_by_project_id(db.get(), payload.project_id.clone()).await?;

                let (status, signature) =
                    match create_raw_transaction(db, fireblocks, rpc, RawTxParams {
                        key: key.clone(),
                        transaction: payload.transaction.context("transaction not found")?,
                        project_id: payload.project_id.clone(),
                        vault: vault.to_string(),
                        tx_type: TxType::MintEdition,
                        treasury_vault_id,
                    })
                    .await
                    {
                        Ok((status, signature)) => (status, signature.to_string()),
                        Err(_) => (TransactionStatus::FAILED, String::new()),
                    };

                emit_mint_retried_event(
                    producer,
                    key,
                    payload.project_id,
                    payload.drop_id,
                    status,
                    signature,
                )
                .await
                .context("failed to emit mint retried event")?;

                Ok(())
            },
            Some(NftEvent::UpdateMetadata(payload)) => {
                let vault =
                    find_vault_id_by_project_id(db.get(), payload.project_id.clone()).await?;

                let (status, signature) =
                    match create_raw_transaction(db, fireblocks, rpc, RawTxParams {
                        key: key.clone(),
                        transaction: payload.transaction.context("transaction not found")?,
                        project_id: payload.project_id.clone(),
                        vault: vault.to_string(),
                        tx_type: TxType::UpdateMetadata,
                        treasury_vault_id,
                    })
                    .await
                    {
                        Ok((status, signature)) => (status, signature.to_string()),
                        Err(_) => (TransactionStatus::FAILED, String::new()),
                    };

                emit_drop_updated_event(producer, key, DropUpdated {
                    project_id: payload.project_id,
                    drop_id: payload.drop_id,
                    status: status as i32,
                    tx_signature: signature,
                })
                .await
                .context("failed to emit drop_created event")?;

                Ok(())
            },
            Some(NftEvent::TransferMint(payload)) => {
                let vault =
                    find_vault_id_by_wallet_address(db.get(), payload.sender.clone()).await?;

                let (_, signature) = create_raw_transaction(db, fireblocks, rpc, RawTxParams {
                    key: key.clone(),
                    transaction: payload
                        .transaction
                        .clone()
                        .context("transaction not found")?,
                    project_id: payload.project_id.clone(),
                    vault,
                    tx_type: TxType::TransferMint,
                    treasury_vault_id,
                })
                .await?;

                emit_mint_transfered_event(producer, key, payload, signature.to_string())
                    .await
                    .context("failed to emit mint_transfered event")?;

                Ok(())
            },

            None => Ok(()),
        },
    }
}
