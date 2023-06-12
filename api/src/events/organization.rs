use fireblocks::objects::vault::{CreateVault, CreateVaultWallet};
use hub_core::{prelude::*, uuid::Uuid};
use sea_orm::{prelude::*, Set};

use super::Processor;
use crate::{
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

#[async_trait]
pub trait OrganizationEventHandler {
    /// Creates a treasury for a project in an organization.
    ///
    /// # Arguments
    /// * conn - A database connection object.
    /// * fireblocks - A Fireblocks client object.
    /// * producer - A Kafka producer object for sending events.
    /// * key - The organization event key.
    /// * project - The project details.
    ///
    /// # Errors
    /// This function may return an error in the following cases:
    /// * Failed to create a vault or wallets in Fireblocks.
    /// * Failed to insert the treasury or project treasuries or wallet record in the local database.
    /// * Failed to send the treasury event using the provided Kafka producer.
    async fn create_project_treasury(
        &self,
        key: OrganizationEventKey,
        project: Project,
    ) -> Result<()>;
}

#[async_trait]
impl OrganizationEventHandler for Processor {
    async fn create_project_treasury(
        &self,
        key: OrganizationEventKey,
        project: Project,
    ) -> Result<()> {
        let conn = self.db.get();
        let user_id = Uuid::from_str(&key.user_id)?;

        let create_vault = CreateVault {
            name: format!("project:{}", project.id.clone()),
            hidden_on_ui: None,
            customer_ref_id: Some(key.user_id),
            auto_fuel: Some(false),
        };

        let vault = self
            .fireblocks
            .client()
            .create()
            .vault(create_vault)
            .await?;

        let treasury = treasuries::ActiveModel {
            vault_id: Set(vault.id.clone()),
            ..Default::default()
        };

        let treasury: treasuries::Model = treasury
            .clone()
            .insert(conn)
            .await
            .context("failed to get treasury record from db")?;

        let project_treasuries_active_model = project_treasuries::ActiveModel {
            project_id: Set(
                Uuid::parse_str(&project.id).context("failed to parse project id to Uuid")?
            ),
            treasury_id: Set(treasury.id),
            ..Default::default()
        };

        project_treasuries_active_model
            .insert(conn)
            .await
            .context("failed to insert project treasuries")?;

        info!("treasury created for project {:?}", project.id);

        // create vault wallets for supported assets
        for asset_type in self.fireblocks.assets().ids() {
            let asset_type = AssetType::from_str(asset_type)?;

            let vault_asset = self
                .fireblocks
                .client()
                .create()
                .wallet(
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
                address: Set(Some(vault_asset.address.clone())),
                created_by: Set(user_id),
                deduction_id: Set(None),
                ..Default::default()
            };

            active_model.insert(conn).await?;

            let proto_blockchain_enum: proto::Blockchain = asset_type.into();
            let project_id = project.id.to_string();

            let event = treasury_events::Event::ProjectWalletCreated(ProjectWallet {
                project_id: project_id.clone(),
                wallet_address: vault_asset.address,
                blockchain: proto_blockchain_enum as i32,
            });

            let event = TreasuryEvents { event: Some(event) };
            let key = TreasuryEventKey {
                id: treasury.id.to_string(),
                user_id: user_id.to_string(),
                project_id,
            };

            self.producer.send(Some(&event), Some(&key)).await?;
        }

        Ok(())
    }
}

impl FromStr for Blockchain {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "SOL" | "SOL_TEST" => Ok(Blockchain::Solana),
            "MATIC" | "MATIC_TEST" => Ok(Blockchain::Polygon),
            "ETH" | "ETH_TEST" => Ok(Blockchain::Ethereum),
            _ => Err(()),
        }
    }
}

impl From<AssetType> for Blockchain {
    fn from(value: AssetType) -> Self {
        match value {
            AssetType::Solana | AssetType::SolanaTest => Blockchain::Solana,
            AssetType::Matic | AssetType::MaticTest => Blockchain::Polygon,
            AssetType::Eth | AssetType::EthTest => Blockchain::Ethereum,
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
