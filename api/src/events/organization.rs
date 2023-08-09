use fireblocks::{
    assets::{ETH, ETH_TEST, MATIC, MATIC_TEST, SOL, SOL_TEST},
    objects::vault::{CreateVault, CreateVaultWallet},
};
use hub_core::{prelude::*, uuid::Uuid};
use sea_orm::{prelude::*, Set};

use super::{Processor, ProcessorError, Result};
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
            .await
            .map_err(ProcessorError::Fireblocks)?;

        let treasury = treasuries::ActiveModel {
            vault_id: Set(vault.id.clone()),
            ..Default::default()
        };

        let treasury: treasuries::Model = treasury.clone().insert(conn).await?;

        let project_treasuries_active_model = project_treasuries::ActiveModel {
            project_id: Set(Uuid::parse_str(&project.id)?),
            treasury_id: Set(treasury.id),
            ..Default::default()
        };

        project_treasuries_active_model.insert(conn).await?;

        for id in self.fireblocks.assets().ids() {
            let asset_type = AssetType::from_str(&id)?;

            let vault_asset = self
                .fireblocks
                .client()
                .create()
                .wallet(
                    treasury.vault_id.clone(),
                    id,
                    CreateVaultWallet {
                        eos_account_name: None,
                    },
                )
                .await
                .map_err(ProcessorError::Fireblocks)?;

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
    type Err = ProcessorError;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            SOL | SOL_TEST => Ok(Blockchain::Solana),
            MATIC | MATIC_TEST => Ok(Blockchain::Polygon),
            ETH | ETH_TEST => Ok(Blockchain::Ethereum),
            v => Err(ProcessorError::InvalidBlockchain(v.into())),
        }
    }
}

impl From<AssetType> for Blockchain {
    fn from(value: AssetType) -> Self {
        match value {
            AssetType::Solana => Blockchain::Solana,
            AssetType::Matic => Blockchain::Polygon,
            AssetType::Eth => Blockchain::Ethereum,
        }
    }
}

impl TryFrom<Blockchain> for AssetType {
    type Error = ProcessorError;

    fn try_from(value: Blockchain) -> Result<Self> {
        match value {
            Blockchain::Solana => Ok(AssetType::Solana),
            Blockchain::Polygon => Ok(AssetType::Matic),
            Blockchain::Ethereum => Ok(AssetType::Eth),
            v @ Blockchain::Unspecified => Err(ProcessorError::InvalidBlockchain(format!("{v:?}"))),
        }
    }
}
