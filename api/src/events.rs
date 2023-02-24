use fireblocks::objects::{
    transaction::{
        CreateTransaction, ExtraParameters, RawMessageData, TransactionOperation, TransferPeerPath,
        UnsignedMessage,
    },
    vault::CreateVault,
};
use hex::FromHex;
use hub_core::{prelude::*, uuid::Uuid};
use sea_orm::{prelude::*, JoinType, QuerySelect, Set};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{signature::Signature, transaction::Transaction};

use crate::{
    db::Connection,
    entities::{customer_treasuries, project_treasuries, treasuries},
    proto::{
        self, customer_events, drop_events,
        organization_events::{self},
        CustomerEventKey, OrganizationEventKey, Project,
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
                    "master edition created".to_string(),
                )
                .await
            },
            Some(drop_events::Event::MintEdition(t)) => {
                create_raw_transaction(k, t, db, fireblocks, rpc, "edition minted".to_string())
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
    k: proto::DropEventKey,
    transaction: proto::Transaction,
    conn: Connection,
    fireblocks: fireblocks::Client,
    rpc: &RpcClient,
    msg: String,
) -> Result<()> {
    let proto::Transaction {
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
        msg, k.user_id, project_id
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

    let transaction = fireblocks.create_transaction(tx).await?;

    let mut tx_details = fireblocks.get_transaction(transaction.id.clone()).await?;

    while tx_details.signed_messages.is_empty() {
        tx_details = fireblocks.get_transaction(transaction.id.clone()).await?;
    }

    let full_sig = tx_details.clone().signed_messages[0]
        .clone()
        .signature
        .full_sig;

    // SIGNATURE LEN = 64 bytes

    let signature_decoded = <[u8; 64]>::from_hex(full_sig)?;

    let signature = Signature::new(&signature_decoded);

    let decoded_message = bincode::deserialize(&serialized_message)?;

    let signed_transaction = Transaction {
        signatures: vec![
            Signature::from_str(payer_signature)?,
            signature,
            Signature::from_str(mint_signature)?,
        ],
        message: decoded_message,
    };

    let res = rpc.send_transaction(&signed_transaction)?;

    info!("{:?} signature {:?}", note, res);

    // emit event

    Ok(())
}
