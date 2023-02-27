use fireblocks::objects::{
    transaction::{
        CreateTransaction, ExtraParameters, RawMessageData, TransactionOperation,
        TransactionStatus, TransferPeerPath, UnsignedMessage,
    },
    vault::CreateVault,
};
use hex::FromHex;
use hub_core::{prelude::*, producer::Producer, tokio::time, uuid::Uuid};
use sea_orm::{prelude::*, JoinType, QuerySelect, Set};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{signature::Signature, transaction::Transaction as SplTransaction};

use crate::{
    db::Connection,
    entities::{customer_treasuries, project_treasuries, treasuries},
    proto::{
        customer_events, drop_events,
        organization_events::{self},
        treasury_events::{self},
        CustomerEventKey, DropEventKey, OrganizationEventKey, Project, Transaction,
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
    rpc: &RpcClient,
    p: Producer<TreasuryEvents>,
) -> Result<()> {
    // match topics
    match msg {
        Services::Customers(k, e) => match e.event {
            Some(customer_events::Event::Created(_)) => {
                create_customer_treasury(k, db, fireblocks).await
            },
            None => Ok(()),
        },
        Services::Organizations(k, e) => match e.event {
            Some(organization_events::Event::ProjectCreated(p)) => {
                create_project_treasury(k, p, db, fireblocks).await
            },
            Some(_) | None => Ok(()),
        },
        Services::Drops(k, e) => match e.event {
            // match topic messages
            Some(drop_events::Event::CreateMasterEdition(t)) => {
                create_raw_transaction(
                    k,
                    t,
                    db,
                    fireblocks,
                    rpc,
                    p,
                    Transactions::CreateMasterEdition,
                )
                .await
            },
            Some(drop_events::Event::MintEdition(t)) => {
                create_raw_transaction(k, t, db, fireblocks, rpc, p, Transactions::MintEdition)
                    .await
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
) -> Result<()> {
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

    Ok(())
}

/// Res
///
/// # Errors
/// This function fails if ...
pub async fn create_customer_treasury(
    k: CustomerEventKey,
    conn: Connection,
    fireblocks: fireblocks::Client,
) -> Result<()> {
    let create_vault = CreateVault {
        name: format!("customer:{}", k.id.clone()),
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

    let customer_am = customer_treasuries::ActiveModel {
        customer_id: Set(Uuid::parse_str(&k.id).context("failed to parse customer id to Uuid")?),
        treasury_id: Set(treasury.id),
        ..Default::default()
    };

    customer_am
        .insert(conn.get())
        .await
        .context("failed to insert customer treasuries")?;

    info!("treasury created for customer {:?}", k.id);

    Ok(())
}

/// Res
///
/// # Errors
/// This function fails if ...
pub async fn create_raw_transaction(
    k: DropEventKey,
    transaction: Transaction,
    conn: Connection,
    fireblocks: fireblocks::Client,
    rpc: &RpcClient,
    producer: Producer<TreasuryEvents>,
    t: Transactions,
) -> Result<()> {
    let Transaction {
        serialized_message,
        signed_message_signatures,
        project_id,
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
            treasuries::Relation::ProjectTreasuries.def(),
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

    let mut interval = time::interval(time::Duration::from_secs(30));

    let transaction = fireblocks.create_transaction(tx).await?;

    let mut tx_details = fireblocks.get_transaction(transaction.id.clone()).await?;

    for _ in 0..10 {
        if !tx_details.clone().signed_messages.is_empty() {
            break;
        }
        interval.tick().await;
        tx_details = fireblocks.get_transaction(transaction.id.clone()).await?;
    }

    let full_sig = tx_details
        .clone()
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

    emit_transaction_status_event(producer, t, k.id, tx_details.status)
        .await
        .context("failed to emit transaction status event")?;

    Ok(())
}

async fn emit_transaction_status_event(
    producer: Producer<TreasuryEvents>,
    t: Transactions,
    id: String,
    status: TransactionStatus,
) -> Result<()> {
    let proto_status = status as i32;
    let event = match t {
        Transactions::CreateMasterEdition => treasury_events::Event::MasterEdition(proto_status),
        Transactions::MintEdition => treasury_events::Event::MintEdition(proto_status),
    };

    let event = TreasuryEvents { event: Some(event) };

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
