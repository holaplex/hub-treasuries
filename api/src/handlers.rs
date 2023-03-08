use std::{str::FromStr, sync::Arc};

use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_poem::{GraphQLRequest, GraphQLResponse};
use fireblocks::objects::transaction::{TransactionStatus, TransactionStatusUpdated};
use hex::FromHex;
use hub_core::{
    anyhow::{Context, Result},
    prelude::bail,
    producer::Producer,
};
use poem::{
    handler,
    web::{Data, Html},
    IntoResponse, Request,
};
use prost::Message;
use sea_orm::EntityTrait;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{signature::Signature, transaction::Transaction};

use crate::{
    db::Connection,
    entities::{prelude::Transactions, wallets::AssetType},
    proto::{treasury_events::Event, TreasuryEventKey, TreasuryEvents},
    AppContext, AppState, UserID,
};

#[handler]
pub fn health() {}

#[handler]
pub fn playground() -> impl IntoResponse {
    Html(playground_source(GraphQLPlaygroundConfig::new("/graphql")))
}

#[handler]
pub async fn graphql_handler(
    Data(state): Data<&AppState>,
    user_id: UserID,
    req: GraphQLRequest,
) -> Result<GraphQLResponse> {
    let context = AppContext::new(state.connection.clone(), user_id);

    Ok(state
        .schema
        .execute(
            req.0
                .data(context)
                .data(state.fireblocks.clone())
                .data(state.producer.clone()),
        )
        .await
        .into())
}

#[handler]
pub async fn fireblocks_webhook_handler(
    db: Data<&Connection>,
    producer: Data<&Producer<TreasuryEvents>>,
    rpc: Data<&Arc<RpcClient>>,
    payload: poem::Body,
    _req: &Request,
) -> Result<()> {
    if let Ok(payload) = payload.into_json::<TransactionStatusUpdated>().await {
        let asset_id = AssetType::from_str(&payload.data.asset_id)?;
        match asset_id {
            AssetType::Solana | AssetType::SolanaTest => {
                process_solana_transaction(&db, &producer, &rpc, payload).await?;
            },
            _ => bail!("unsupported asset id"),
        }
    }

    Ok(())
}

/// Res
///
/// # Errors
/// This function fails if ...
pub async fn process_solana_transaction(
    db: &Connection,
    producer: &Producer<TreasuryEvents>,
    rpc: &Arc<RpcClient>,
    payload: TransactionStatusUpdated,
) -> Result<()> {
    let tx = Transactions::find_by_id(payload.data.id)
        .one(db.get())
        .await?
        .context("no signatures found")?;

    let mut signature = Signature::default();
    let mut status = payload.data.status;

    if payload.data.status == TransactionStatus::COMPLETED {
        let message = payload
            .data
            .signed_messages
            .get(0)
            .context("failed to get signed message")?
            .content
            .clone();

        let message_bytes = hex::decode(message)?;

        let message = bincode::deserialize_from(message_bytes.as_slice())?;

        let full_sig = payload
            .data
            .signed_messages
            .get(0)
            .context("failed to get signed message response")?
            .clone()
            .signature
            .full_sig;

        let signature_decoded = <[u8; 64]>::from_hex(full_sig)?;

        signature = Signature::new(&signature_decoded);

        let mut signatures = tx
            .signed_message_signatures
            .iter()
            .map(|s| Signature::from_str(s).map_err(Into::into))
            .collect::<Result<Vec<Signature>>>()
            .context("failed to parse signatures")?;

        signatures.push(signature);

        let transaction = Transaction {
            signatures,
            message,
        };

        if rpc.send_transaction(&transaction).await.is_err() {
            status = TransactionStatus::FAILED;
        }
    }

    emit_event(
        producer,
        tx.event_id,
        tx.event_payload,
        status,
        Some(signature.to_string()),
    )
    .await?;

    Ok(())
}

/// Res
///
/// # Errors
/// This function fails if ...
async fn emit_event(
    producer: &Producer<TreasuryEvents>,
    id: Vec<u8>,
    payload: Vec<u8>,
    status: TransactionStatus,
    signature: Option<String>,
) -> Result<()> {
    let key: TreasuryEventKey = TreasuryEventKey::decode(id.as_slice())?;
    let treasury_event: TreasuryEvents = TreasuryEvents::decode(payload.as_slice())?;

    match treasury_event.event.clone() {
        Some(Event::DropMinted(mut e)) => {
            e.status = status as i32;
            e.tx_signature = signature.unwrap_or_default();
        },
        Some(Event::DropCreated(mut e)) => {
            e.status = status as i32;
            e.tx_signature = signature.unwrap_or_default();
        },
        None | Some(_) => (),
    }

    producer
        .send(Some(&treasury_event), Some(&key))
        .await
        .map_err(Into::into)
}
