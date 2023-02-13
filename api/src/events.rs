use fireblocks::objects::{
    transaction::{
        CreateTransaction, RawMessageData, TransactionOperation, TransferPeerPath, UnsignedMessage,
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
    entities::{project_treasuries, treasuries},
    proto::{
        self, drop_events,
        organization_events::{self},
        OrganizationEventKey, Project,
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
        Services::Organizations(k, e) => match e.event {
            // match topic messages
            Some(organization_events::Event::ProjectCreated(p)) => {
                create_treasury(k, p, db, fireblocks).await
            },
            Some(_) | None => Ok(()),
        },
        Services::Drops(k, e) => match e.event {
            // match topic messages
            Some(drop_events::Event::CreateMasterEdition(t)) => {
                create_master_edition(k, t, db, fireblocks, rpc).await
            },

            None => Ok(()),
        },
    }
}

/// Res
///
/// # Errors
/// This function fails if ...
pub async fn create_treasury(
    k: OrganizationEventKey,
    project: Project,
    conn: Connection,
    fireblocks: fireblocks::Client,
) -> Result<()> {
    let create_vault = CreateVault {
        name: project.id.clone(),
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
pub async fn create_master_edition(
    k: proto::DropEventKey,
    transaction: proto::Transaction,
    conn: Connection,
    fireblocks: fireblocks::Client,
    rpc: &RpcClient,
) -> Result<()> {
    let proto::Transaction {
        serialized_message,
        signed_message_signature,
        project_id,
    } = transaction;

    let project = Uuid::parse_str(&project_id)?;

    let vault = treasuries::Entity::find()
        .join(
            JoinType::InnerJoin,
            project_treasuries::Relation::Treasuries.def(),
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
        extra_parameters: Some(RawMessageData {
            messages: vec![UnsignedMessage {
                content: hex::encode(&serialized_message),
            }],
        }),
        note: Some(format!(
            "CreateMasterEdition by {:?} for project {:?}",
            k.user_id, project_id
        )),
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
        signatures: vec![signature, Signature::from_str(&signed_message_signature)?],
        message: decoded_message,
    };

    let res = rpc.send_transaction(&signed_transaction)?;

    debug!("MasterEdition sucessfully created {:?}", res);

    Ok(())
}
