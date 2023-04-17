use fireblocks::objects::vault::CreateVault;
use hub_core::{prelude::*, producer::Producer, uuid::Uuid};
use sea_orm::{prelude::*, Set};

use crate::{
    db::Connection,
    entities::{customer_treasuries, treasuries},
    proto::{
        treasury_events::{self, CustomerTreasury},
        Customer, CustomerEventKey, TreasuryEventKey, TreasuryEvents,
    },
};

/// Creates a customer treasury in the Fireblocks system and records the treasury details in the local database.
///
/// # Arguments
/// * conn - A database connection object.
/// * fireblocks - A Fireblocks client object.
/// * producer - A Kafka producer object for sending events.
/// * key - The customer event key.
/// * customer - The customer details.
///
/// # Errors
/// This function may return an error in the following cases:
/// * Failed to create the vault in Fireblocks.
/// * Failed to insert the treasury record in the local database.
/// * Failed to parse the customer ID to a UUID.
/// * Failed to insert the customer treasuries record in the local database.
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
                customer_id: key.id.clone(),
                project_id: customer.project_id,
            },
        )),
    };

    let key = TreasuryEventKey {
        id: treasury.id.to_string(),
        user_id: key.id,
    };

    producer.send(Some(&event), Some(&key)).await?;

    Ok(())
}
