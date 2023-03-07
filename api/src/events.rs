use fireblocks::objects::{
    transaction::{
        CreateTransaction, CreateTransactionResponse, ExtraParameters, RawMessageData,
        TransactionOperation, TransferPeerPath, UnsignedMessage,
    },
    vault::{CreateVault, CreateVaultWallet},
};
use hub_core::{prelude::*, producer::Producer, uuid::Uuid};
use sea_orm::{prelude::*, JoinType, QuerySelect, Set};

use crate::{
    db::Connection,
    entities::{
        customer_treasuries, project_treasuries,
        sea_orm_active_enums::EventType,
        transactions, treasuries,
        wallets::{self, AssetType},
    },
    objects::{blockchain::Blockchain, BLOCKCHAIN_ASSET_IDS},
    proto::{
        self, customer_events, nft_events,
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
                let (transaction, signatures) = create_raw_transaction(
                    key.clone(),
                    payload.transaction.context("transaction not found")?,
                    payload.project_id.clone(),
                    db.clone(),
                    fireblocks,
                    Transactions::CreateMasterEdition,
                )
                .await?;

                let event = TreasuryEvents {
                    event: Some(treasury_events::Event::DropCreated(DropCreated {
                        project_id: payload.project_id,
                        status: transaction.status as i32,
                        tx_signature: String::default(),
                    })),
                };

                let key = TreasuryEventKey { id: key.id };

                let tx_am = transactions::ActiveModel {
                    id: Set(transaction.id),
                    status: Set(transaction.status.into()),
                    signed_message_signatures: Set(signatures),
                    full_signature: Set(None),
                    event_type: Set(EventType::CreateDrop),
                    event_id: Set(key.encode_to_vec()),
                    event_payload: Set(event.encode_to_vec()),
                };

                tx_am.insert(db.get()).await?;

                Ok(())
            },
            Some(nft_events::Event::MintDrop(payload)) => {
                let (transaction, signatures) = create_raw_transaction(
                    key.clone(),
                    payload.transaction.context("transaction not found")?,
                    payload.project_id.clone(),
                    db.clone(),
                    fireblocks,
                    Transactions::MintEdition,
                )
                .await?;

                let event = TreasuryEvents {
                    event: Some(treasury_events::Event::DropMinted(DropMinted {
                        project_id: payload.project_id,
                        drop_id: payload.drop_id,
                        status: transaction.status as i32,
                        tx_signature: String::default(),
                    })),
                };

                let key = TreasuryEventKey { id: key.id };

                let tx_am = transactions::ActiveModel {
                    id: Set(transaction.id),
                    status: Set(transaction.status.into()),
                    signed_message_signatures: Set(signatures),
                    full_signature: Set(None),
                    event_type: Set(EventType::CreateDrop),
                    event_id: Set(key.encode_to_vec()),
                    event_payload: Set(event.encode_to_vec()),
                };

                tx_am.insert(db.get()).await?;

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
    t: Transactions,
) -> Result<(CreateTransactionResponse, Vec<String>)> {
    let Transaction {
        serialized_message,
        signed_message_signatures,
        blockchain,
    } = transaction;

    let proto_blockchain_enum =
        proto::Blockchain::from_i32(blockchain).context("failed to parse to blockchain enum")?;

    let ids = BLOCKCHAIN_ASSET_IDS
        .get(&proto_blockchain_enum.try_into()?)
        .context("failed to get asset ids")?
        .clone();

    let project = Uuid::parse_str(&project_id)?;
    let note = Some(format!(
        "Event:{:?},ID:{:?},ProjectID:{:?}",
        t.to_string(),
        k.user_id,
        project_id
    ));

    let treasury_model = treasuries::Entity::find()
        .join(
            JoinType::InnerJoin,
            treasuries::Relation::ProjectTreasuries.def(),
        )
        .filter(project_treasuries::Column::ProjectId.eq(project))
        .one(conn.get())
        .await?
        .context("treasury not found in database")?;

    let wallet = wallets::Entity::find()
        .filter(
            wallets::Column::AssetId
                .is_in(ids)
                .and(wallets::Column::TreasuryId.eq(treasury_model.id)),
        )
        .one(conn.get())
        .await?
        .context("treasury not found in database")?;

    let tx = CreateTransaction {
        asset_id: wallet.asset_id.into(),
        operation: TransactionOperation::RAW,
        source: TransferPeerPath {
            peer_type: "VAULT_ACCOUNT".to_string(),
            id: treasury_model.vault_id.to_string(),
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

    Ok((transaction, signed_message_signatures))
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

impl TryFrom<proto::Blockchain> for Blockchain {
    type Error = Error;

    fn try_from(value: proto::Blockchain) -> Result<Self> {
        match value {
            proto::Blockchain::Unspecified => Err(anyhow!("Invalid enum variant")),
            proto::Blockchain::Solana => Ok(Self::Solana),
            proto::Blockchain::Polygon => Ok(Self::Polygon),
            proto::Blockchain::Ethereum => Ok(Self::Ethereum),
        }
    }
}
