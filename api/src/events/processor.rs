use fireblocks::Fireblocks;
use hub_core::{prelude::*, producer::Producer, uuid::Uuid};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use super::{
    customer::CustomerEventHandler, organization::OrganizationEventHandler, polygon::Polygon,
    signer::Transactions, solana::Solana,
};
use crate::{
    db::Connection,
    entities::{project_treasuries, treasuries},
    proto::{
        customer_events::Event as CustomerEvent, organization_events::Event as OrganizationEvent,
        polygon_nft_events::Event as PolygonNftEvent, solana_nft_events::Event as SolanaNftEvent,
        TreasuryEvents,
    },
    Services,
};

#[derive(Clone)]
pub struct Processor {
    pub db: Connection,
    pub fireblocks: Fireblocks,
    pub producer: Producer<TreasuryEvents>,
}

impl Processor {
    #[must_use]
    pub fn new(db: Connection, producer: Producer<TreasuryEvents>, fireblocks: Fireblocks) -> Self {
        Self {
            db,
            fireblocks,
            producer,
        }
    }

    /// Processes a message from the event stream.
    /// # Errors
    /// Returns an error if the message cannot be processed.
    pub async fn process(&self, msg: Services) -> Result<()> {
        // match topics
        match msg {
            Services::Customers(key, e) => match e.event {
                Some(CustomerEvent::Created(customer)) => {
                    self.customer().create_treasury(key, customer).await
                },
                Some(_) | None => Ok(()),
            },
            Services::Organizations(key, e) => match e.event {
                Some(OrganizationEvent::ProjectCreated(project)) => {
                    self.organization()
                        .create_project_treasury(key, project)
                        .await
                },
                Some(_) | None => Ok(()),
            },
            Services::Polygon(key, e) => {
                let polygon = self.polygon();
                let signer = polygon.signer();

                #[allow(clippy::single_match)]
                match e.event {
                    Some(PolygonNftEvent::SubmitCreateDropTxn(payload)) => {
                        signer.create_drop(key.clone(), payload).await?;
                    },

                    None => (),
                }
                Ok(())
            },
            Services::Solana(key, e) => {
                let conn = self.db.get();
                let vault_id =
                    Self::find_vault_id_by_project_id(conn, key.project_id.clone()).await?;
                let solana = self.solana();
                let signer = solana.signer(vault_id);

                match e.event {
                    Some(SolanaNftEvent::SignCreateDrop(payload)) => {
                        signer.create_drop(key.clone(), payload).await?;
                    },
                    Some(SolanaNftEvent::SignUpdateDrop(payload)) => {
                        signer.update_drop(key.clone(), payload).await?;
                    },
                    Some(SolanaNftEvent::SignMintDrop(payload)) => {
                        signer.mint_drop(key.clone(), payload).await?;
                    },
                    Some(SolanaNftEvent::SignTransferAsset(payload)) => {
                        signer.transfer_asset(key.clone(), payload).await?;
                    },
                    Some(SolanaNftEvent::SignRetryCreateDrop(payload)) => {
                        signer.retry_mint_drop(key.clone(), payload).await?;
                    },
                    Some(SolanaNftEvent::SignRetryMintDrop(payload)) => {
                        signer.retry_mint_drop(key.clone(), payload).await?;
                    },
                    _ => (),
                };

                Ok(())
            },
        }
    }

    fn customer(&self) -> impl CustomerEventHandler {
        self.clone()
    }

    fn organization(&self) -> impl OrganizationEventHandler {
        self.clone()
    }

    fn solana(&self) -> Solana {
        Solana::new(self.fireblocks.clone(), self.producer.clone())
    }

    fn polygon(&self) -> Polygon {
        Polygon::new(self.fireblocks.clone(), self.producer.clone())
    }

    async fn find_vault_id_by_project_id(
        db: &DatabaseConnection,
        project: String,
    ) -> Result<String> {
        let project = Uuid::from_str(&project)?;

        let (_, t) = project_treasuries::Entity::find()
            .find_also_related(treasuries::Entity)
            .filter(project_treasuries::Column::ProjectId.eq(project))
            .one(db)
            .await?
            .context("treasury not found in database")?;

        let t = t.ok_or(anyhow!("treasury not found"))?;

        Ok(t.vault_id)
    }
}
