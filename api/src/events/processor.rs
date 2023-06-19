use fireblocks::Fireblocks;
use hub_core::{prelude::*, producer::Producer};

use super::{
    customer::CustomerEventHandler, organization::OrganizationEventHandler, polygon::Polygon,
    signer::Transactions, solana::Solana,
};
use crate::{
    db::Connection,
    proto::{
        customer_events::Event as CustomerEvent, organization_events::Event as OrganizationEvent,
        solana_nft_events::Event as SolanaNftEvent, TreasuryEvents,
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
            Services::Polygon(key, e) => self.polygon().process(key, e).await,
            Services::Solana(key, e) => {
                let solana = self.solana();

                match e.event {
                    Some(SolanaNftEvent::CreateDropSigningRequested(payload)) => {
                        solana.create_drop(key.clone(), payload).await?;
                    },
                    Some(SolanaNftEvent::UpdateDropSigningRequested(payload)) => {
                        solana.update_drop(key.clone(), payload).await?;
                    },
                    Some(SolanaNftEvent::MintDropSigningRequested(payload)) => {
                        solana.mint_drop(key.clone(), payload).await?;
                    },
                    Some(SolanaNftEvent::TransferAssetSigningRequested(payload)) => {
                        solana.transfer_asset(key.clone(), payload).await?;
                    },
                    Some(SolanaNftEvent::RetryCreateDropSigningRequested(payload)) => {
                        solana.retry_mint_drop(key.clone(), payload).await?;
                    },
                    Some(SolanaNftEvent::RetryMintDropSigningRequested(payload)) => {
                        solana.retry_mint_drop(key.clone(), payload).await?;
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
        Solana::new(
            self.fireblocks.clone(),
            self.producer.clone(),
            self.db.clone(),
        )
    }

    fn polygon(&self) -> Polygon {
        Polygon::new(
            self.fireblocks.clone(),
            self.producer.clone(),
            self.db.clone(),
        )
    }
}
