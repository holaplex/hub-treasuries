use fireblocks::objects::{
    transaction::{
        CreateTransaction, ExtraParameters, RawMessageData, TransactionOperation, TransferPeerPath,
        UnsignedMessage,
    },
    vault::CreateVault,
};
use hex::FromHex;
use hub_core::{prelude::*, uuid::Uuid};
use sea_orm::{prelude::*, Set};
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
            Some(_) => Ok(()),

            None => Ok(()),
        },
        Services::Drops(k, e) => match e.event {
            // match topic messages
            Some(drop_events::Event::MintEditionTransaction(t)) => {
                mint_edition(k, t, db, fireblocks, rpc).await
            },

            None => Ok(()),
        },
    }
}

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

pub async fn mint_edition(
    k: proto::DropEventKey,
    transaction: proto::Transaction,
    conn: Connection,
    fireblocks: fireblocks::Client,
    rpc: &RpcClient,
) -> Result<()> {
    let proto::Transaction {
        serialized_message,
        signed_message_signature,
        hashed_message,
        project_id,
        organization_id,
        blockhash,
    } = transaction;

    let tx = CreateTransaction {
        asset_id: "SOL_TEST".to_string(),
        operation: TransactionOperation::RAW,
        source: TransferPeerPath {
            peer_type: "VAULT_ACCOUNT".to_string(),
            id: "6".to_string(),
        },
        destination: None,
        destinations: None,
        treat_as_gross_amount: None,
        customer_ref_id: None,
        amount: "0".to_string(),
        extra_parameters: Some(ExtraParameters::RawMessageData(RawMessageData {
            messages: vec![UnsignedMessage {
                content: hashed_message,
            }],
        })),
        note: Some("solana edition minting".to_string()),
    };

    let transaction = fireblocks.create_transaction(tx).await?;

    let mut tx_details = fireblocks.get_transaction(transaction.id.clone()).await?;

    while tx_details.signed_messages.len() == 0 {
        tx_details = fireblocks.get_transaction(transaction.id.clone()).await?;
    }

    debug!("{:?}", tx_details.signed_messages);

    let full_sig = tx_details.clone().signed_messages[0]
        .clone()
        .signature
        .full_sig;

    // SIGNATURE LEN = 64 bytes

    let signature_decoded = <[u8; 64]>::from_hex(full_sig)?;

    let signature = Signature::new(&signature_decoded);

    debug!("signature {:?}", signature);
    let decoded_message: solana_sdk::message::Message = bincode::deserialize(&serialized_message)?;

    let signed_transaction = Transaction {
        signatures: vec![signature, Signature::from_str(&signed_message_signature)?],
        message: decoded_message,
    };

    let res = rpc.send_transaction(&signed_transaction);

    debug!("response {:?}", res);

    Ok(())
}
