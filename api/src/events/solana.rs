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
            signed_transaction::Transaction, SignedTransaction, SolanaSignedTransaction,
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
        sign_transaction(self, TxType::CreateDrop, key, payload).await
    }

    async fn update_drop(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaTransaction,
    ) -> Result<SignedTransaction> {
        sign_transaction(self, TxType::UpdateMetadata, key, payload).await
    }

    async fn mint_drop(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaTransaction,
    ) -> Result<SignedTransaction> {
        sign_transaction(self, TxType::MintEdition, key, payload).await
    }

    async fn transfer_asset(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaTransaction,
    ) -> Result<SignedTransaction> {
        sign_transaction(self, TxType::TransferMint, key, payload).await
    }

    async fn retry_create_drop(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaTransaction,
    ) -> Result<SignedTransaction> {
        sign_transaction(self, TxType::CreateDrop, key, payload).await
    }

    async fn retry_mint_drop(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaTransaction,
    ) -> Result<SignedTransaction> {
        sign_transaction(self, TxType::MintEdition, key, payload).await
    }
}

async fn sign_transaction(
    signer: &Signer,
    tx_type: TxType,
    key: SolanaNftEventKey,
    mut payload: SolanaTransaction,
) -> Result<SignedTransaction> {
    let note = format!(
        "{:?} by {:?} for project {:?}",
        tx_type, key.user_id, key.project_id
    );

    let signature = signer
        .sign_message(note, payload.serialized_message.clone())
        .await?;

    payload
        .signed_message_signatures
        .push(bs58::encode(signature).into_string());

    Ok(SignedTransaction {
        transaction: Some(Transaction::Solana(SolanaSignedTransaction {
            serialized_message: payload.serialized_message,
            signed_message_signatures: payload.signed_message_signatures,
            project_id: key.project_id,
        })),
    })
}
