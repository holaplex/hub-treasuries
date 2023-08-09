use fireblocks::objects::vault::CreateVault;
use hub_core::{prelude::*, uuid::Uuid};
use sea_orm::{prelude::*, Set};

use super::{Processor, Result};
use crate::{
    entities::{customer_treasuries, treasuries},
    events::ProcessorError,
    proto::{
        treasury_events::{self, CustomerTreasury},
        Customer, CustomerEventKey, TreasuryEventKey, TreasuryEvents,
    },
};

impl Processor {
    pub(super) async fn create_treasury(
        &self,
        key: CustomerEventKey,
        customer: Customer,
    ) -> Result<()> {
        let conn = self.db.get();
        let create_vault = CreateVault {
            name: format!("customer:{}", key.id.clone()),
            hidden_on_ui: None,
            customer_ref_id: None,
            auto_fuel: Some(false),
        };

        let vault = self
            .fireblocks
            .client()
            .create()
            .vault(create_vault)
            .await
            .map_err(ProcessorError::Fireblocks)?;

        info!("vault created {:?}", vault);

        let treasury = treasuries::ActiveModel {
            vault_id: Set(vault.id.clone()),
            ..Default::default()
        };

        let treasury: treasuries::Model = treasury.clone().insert(conn).await?;

        let project_id = Uuid::from_str(&customer.project_id)?;

        let customer_am = customer_treasuries::ActiveModel {
            customer_id: Set(Uuid::parse_str(&key.id)?),
            treasury_id: Set(treasury.id),
            project_id: Set(project_id),
            ..Default::default()
        };

        customer_am.insert(conn).await?;

        info!("treasury created for customer {:?}", key.id);

        let event = TreasuryEvents {
            event: Some(treasury_events::Event::CustomerTreasuryCreated(
                CustomerTreasury {
                    customer_id: key.id.clone(),
                    project_id: project_id.to_string(),
                },
            )),
        };

        let key = TreasuryEventKey {
            id: treasury.id.to_string(),
            user_id: key.id,
            project_id: project_id.to_string(),
        };

        self.producer.send(Some(&event), Some(&key)).await?;

        Ok(())
    }
}
