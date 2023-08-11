use fireblocks::Fireblocks;
use hub_core::{
    prelude::*,
    producer::{Producer, SendError},
    thiserror, uuid,
};
use sea_orm::DbErr;

use super::{polygon::Polygon, solana::Solana};
use crate::{
    db::Connection,
    entities::wallets::TryIntoAssetTypeError,
    proto::{
        customer_events::Event as CustomerEvent, organization_events::Event as OrganizationEvent,
        TreasuryEvents,
    },
    Services,
};

#[derive(Debug, Clone, Copy)]
pub enum EcdsaSignatureScalar {
    R,
    S,
    V,
}

impl fmt::Display for EcdsaSignatureScalar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::R => "r",
            Self::S => "s",
            Self::V => "v",
        })
    }
}

#[derive(Debug, thiserror::Error, Triage)]
pub enum ProcessorError {
    #[error("No treasury found for wallet address {0:?}")]
    InvalidWalletAddress(String),
    #[error("Invalid blockchain {0:?}")]
    InvalidBlockchain(String),
    #[error("Missing {0} scalar of ECDSA signature")]
    IncompleteEcdsaSignature(EcdsaSignatureScalar),
    #[error("Field permit_token_transfer_txn not found in event payload")]
    MissingPermitTokenTransferTxn,
    #[error("Field safe_transfer_from_txn not found in event payload")]
    MissingSafeTransferFromTxn,
    #[error("Signed message not found in transaction response")]
    MissingSignedMessage,

    #[error("Invalid ECDSA pubkey recovery scalar")]
    #[permanent]
    InvalidEcdsaPubkeyRecovery(#[source] std::num::TryFromIntError),
    #[error("Fireblocks error")]
    #[transient]
    Fireblocks(#[source] Error),
    #[error("Invalid UUID")]
    InvalidUuid(#[from] uuid::Error),
    #[error("Invalid hex string")]
    #[permanent]
    InvalidHex(#[from] hex::FromHexError),
    #[error("Invalid asset type")]
    #[permanent]
    InvalidAssetType(#[from] TryIntoAssetTypeError),
    #[error("Database error")]
    DbError(#[from] DbErr),
    #[error("Error sending message")]
    SendError(#[from] SendError),
}

pub type Result<T> = std::result::Result<T, ProcessorError>;

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
                Some(CustomerEvent::Created(customer)) => self.create_treasury(key, customer).await,
                Some(_) | None => Ok(()),
            },
            Services::Organizations(key, e) => match e.event {
                Some(OrganizationEvent::ProjectCreated(project)) => {
                    self.create_project_treasury(key, project).await
                },
                Some(_) | None => Ok(()),
            },
            Services::Polygon(key, e) => self.polygon().process(key, e).await,
            Services::Solana(key, e) => self.solana().process(key, e).await,
        }
    }

    #[inline]
    fn solana(&self) -> Solana {
        Solana::new(self)
    }

    #[inline]
    fn polygon(&self) -> Polygon {
        Polygon::new(self)
    }
}
