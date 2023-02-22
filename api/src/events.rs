use fireblocks::objects::vault::{CreateVault, CreateVaultWallet};
use hub_core::{prelude::*, uuid::Uuid};
use sea_orm::{prelude::*, Set};

use crate::{
    db::Connection,
    entities::{customer_treasuries, treasuries},
    proto::{customer_events, CustomerEventKey},
    Services,
};

/// Res
///
/// # Errors
/// This function fails if ...
pub async fn process(msg: Services, db: Connection, fireblocks: fireblocks::Client) -> Result<()> {
    // match topics
    match msg {
        Services::Customers(k, e) => match e.event {
            Some(customer_events::Event::Created(_)) => {
                create_customer_treasury(k, db, fireblocks).await
            },
            None => Ok(()),
        },
        Services::Organizations(..) => Ok(()),
    }
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
        name: k.id.clone(),
        hidden_on_ui: None,
        customer_ref_id: None,
        auto_fuel: Some(false),
    };

    let vault = fireblocks.create_vault(create_vault).await?;

    info!("vault created {:?}", vault);

    let wallet = fireblocks
        .create_wallet(vault.id.clone(), "SOL_TEST".to_owned(), CreateVaultWallet {
            eos_account_name: None,
        })
        .await?;

    info!("wallet created for customer {:?}", wallet);

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
