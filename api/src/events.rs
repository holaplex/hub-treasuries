use fireblocks::objects::vault::CreateVault;
use hub_core::{prelude::*, uuid::Uuid};
use sea_orm::{prelude::*, Set};

use crate::{
    db::Connection,
    entities::{project_treasuries, treasuries},
    proto::{event::EventPayload, Key, Project},
    Services,
};

pub async fn process(msg: Services, db: Connection, fireblocks: fireblocks::Client) -> Result<()> {
    // match topics
    match msg {
        Services::Org(k, e) => match e.event_payload {
            // match topic messages
            Some(EventPayload::ProjectCreated(p)) => create_treasury(k, p, db, fireblocks).await,
            Some(_) | None => Ok(()),
        },
    }
}

pub async fn create_treasury(
    k: Key,
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
    let organization_id = Uuid::from_str(&project.organization_id)?;

    let treasury = treasuries::ActiveModel {
        vault_id: Set(vault.id.clone()),
        organization_id: Set(organization_id),
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
