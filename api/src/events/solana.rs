use fireblocks::Fireblocks;
use hub_core::{prelude::*, producer::Producer};

use super::{
    emitter::Emitter,
    signer::{Signer, TransactionSigner},
};
use crate::{
    db::Connection,
    entities::sea_orm_active_enums::TxType,
    proto::{
        treasury_events::{
            SignedTransaction,
        },
        SolanaNftEventKey, SolanaTransaction, TreasuryEvents,
    },
};

pub struct Solana {
    fireblocks: Fireblocks,
    db: Connection,
    emitter: Emitter,
}

impl Solana {
    #[must_use]
    pub fn new(fireblocks: Fireblocks, db: Connection, producer: Producer<TreasuryEvents>) -> Self {
        let emitter = Emitter::new(producer);

        Self {
            fireblocks,
            db,
            emitter,
        }
    }

    #[must_use]
    pub fn signer(&self, vault_id: String) -> Signer {
        Signer::new(self.fireblocks.clone(), self.db.clone(), vault_id)
    }

    #[must_use]
    pub fn event(&self) -> Emitter {
        self.emitter.clone()
    }
}

#[async_trait]
impl TransactionSigner<SolanaNftEventKey, SolanaTransaction> for Signer {
    async fn create_drop(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaTransaction,
    ) -> Result<SignedTransaction> {
        self.sign_transaction(TxType::CreateDrop, key, payload).await
    }

    async fn update_drop(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaTransaction,
    ) -> Result<SignedTransaction> {
        self.sign_transaction(TxType::UpdateMetadata, key, payload).await
    }

    async fn mint_drop(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaTransaction,
    ) -> Result<SignedTransaction> {
        self.sign_transaction(TxType::MintEdition, key, payload).await
    }

    async fn transfer_asset(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaTransaction,
    ) -> Result<SignedTransaction> {
        self.sign_transaction(TxType::TransferMint, key, payload).await
    }

    async fn retry_create_drop(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaTransaction,
    ) -> Result<SignedTransaction> {
        self.sign_transaction(TxType::CreateDrop, key, payload).await
    }

    async fn retry_mint_drop(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaTransaction,
    ) -> Result<SignedTransaction> {
        self.sign_transaction(TxType::MintEdition, key, payload).await
    }
}

