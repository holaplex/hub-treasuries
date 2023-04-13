use fireblocks::objects::vault::{CreateVault, CreateVaultWallet};
use hub_core::{prelude::*, producer::Producer, uuid::Uuid};
use sea_orm::{prelude::*, Set};

use crate::{
    db::Connection,
    entities::{
        project_treasuries, treasuries,
        wallets::{self, AssetType},
    },
    proto::{
        self,
        treasury_events::{self, ProjectWallet},
        Blockchain, OrganizationEventKey, Project, TreasuryEventKey, TreasuryEvents,
    },
};

///  creates a treasury for a project in an organization
///
/// # Errors
/// This function may return an error in the following cases:
/// * Failed to create a vault or wallets in Fireblocks.
/// * Failed to insert the treasury or project treasuries or wallet record in the local database.
/// * Failed to send the treasury event using the provided Kafka producer.
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

        let proto_blockchain_enum: proto::Blockchain = asset_type.into();

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

impl TryFrom<Blockchain> for Vec<AssetType> {
    type Error = Error;

    fn try_from(value: Blockchain) -> Result<Self> {
        match value {
            Blockchain::Solana => Ok(vec![AssetType::Solana, AssetType::SolanaTest]),
            Blockchain::Polygon => Ok(vec![AssetType::Matic, AssetType::MaticTest]),
            Blockchain::Ethereum => Ok(vec![AssetType::Eth, AssetType::EthTest]),
            Blockchain::Unspecified => Err(anyhow!("Unspecified blockchain")),
        }
    }
}
