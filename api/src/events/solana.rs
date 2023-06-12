use fireblocks::Fireblocks;
use futures::future::ready;
use hub_core::{prelude::*, producer::Producer};
use solana_sdk::pubkey::Pubkey;

use super::signer::{Events, Sign, Transactions};
use crate::{
    db::Connection,
    entities::sea_orm_active_enums::TxType,
    proto::{
        treasury_events::{Event, SolanaSignedTxn},
        SolanaNftEventKey, SolanaPendingTransaction, TreasuryEventKey, TreasuryEvents,
    },
};

pub struct Solana {
    fireblocks: Fireblocks,
    producer: Producer<TreasuryEvents>,
    db: Connection,
}

impl Solana {
    #[must_use]
    pub fn new(fireblocks: Fireblocks, producer: Producer<TreasuryEvents>, db: Connection) -> Self {
        Self {
            fireblocks,
            producer,
            db,
        }
    }

    pub fn is_public_key(test_case: &str) -> bool {
        Pubkey::from_str(test_case).is_ok()
    }
}

#[async_trait]
impl Transactions<SolanaNftEventKey, SolanaPendingTransaction, SolanaSignedTxn> for Solana {
    async fn create_drop(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaPendingTransaction,
    ) -> Result<SolanaSignedTxn> {
        let tx = self
            .send_transaction(TxType::CreateDrop, key.clone(), payload)
            .await?;

        self.on_create_drop(key, tx.clone()).await?;

        Ok(tx)
    }

    async fn update_drop(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaPendingTransaction,
    ) -> Result<SolanaSignedTxn> {
        let tx = self
            .send_transaction(TxType::UpdateMetadata, key.clone(), payload)
            .await?;

        self.on_update_drop(key, tx.clone()).await?;

        Ok(tx)
    }

    async fn mint_drop(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaPendingTransaction,
    ) -> Result<SolanaSignedTxn> {
        let tx = self
            .send_transaction(TxType::MintEdition, key.clone(), payload)
            .await?;

        self.on_mint_drop(key, tx.clone()).await?;
        Ok(tx)
    }

    async fn transfer_asset(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaPendingTransaction,
    ) -> Result<SolanaSignedTxn> {
        let tx = self
            .send_transaction(TxType::TransferMint, key.clone(), payload)
            .await?;

        self.on_transfer_asset(key, tx.clone()).await?;
        Ok(tx)
    }

    async fn retry_create_drop(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaPendingTransaction,
    ) -> Result<SolanaSignedTxn> {
        let tx = self
            .send_transaction(TxType::CreateDrop, key.clone(), payload)
            .await?;

        self.on_retry_create_drop(key, tx.clone()).await?;
        Ok(tx)
    }

    async fn retry_mint_drop(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaPendingTransaction,
    ) -> Result<SolanaSignedTxn> {
        let tx = self
            .send_transaction(TxType::MintEdition, key.clone(), payload)
            .await?;

        self.on_retry_mint_drop(key, tx.clone()).await?;
        Ok(tx)
    }
}

#[async_trait]
impl Sign<SolanaNftEventKey, SolanaPendingTransaction, SolanaSignedTxn> for Solana {
    const ASSET_ID: &'static str = "SOL";

    async fn send_transaction(
        &self,
        tx_type: TxType,
        key: SolanaNftEventKey,
        SolanaPendingTransaction {
            serialized_message,
            signatures_or_signers_public_keys,
        }: SolanaPendingTransaction,
    ) -> Result<SolanaSignedTxn> {
        let conn = self.db.get();
        let note = format!(
            "{:?} by {:?} for project {:?}",
            tx_type, key.user_id, key.project_id,
        );

        let mut fireblocks_requests = Vec::new();

        for req_sig in signatures_or_signers_public_keys {
            if Self::is_public_key(&req_sig) {
                let vault_id = Self::find_vault_id_by_wallet_address(conn, req_sig).await?;
                let fireblocks_request = Self::request_and_wait_signature_from_fireblocks(
                    &self.fireblocks,
                    note.clone(),
                    serialized_message.clone(),
                    vault_id,
                );

                fireblocks_requests.push(fireblocks_request);
            } else {
                fireblocks_requests.push(Box::pin(ready(Ok(req_sig))));
            }
        }

        let signed_message_signatures = futures::future::try_join_all(fireblocks_requests).await?;

        Ok(SolanaSignedTxn {
            serialized_message,
            signed_message_signatures,
        })
    }
}

#[async_trait]
impl Events<SolanaNftEventKey, SolanaSignedTxn> for Solana {
    async fn on_create_drop(&self, key: SolanaNftEventKey, tx: SolanaSignedTxn) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::SolanaCreateDropSigned(tx)),
        };

        self.producer.send(Some(&event), Some(&key.into())).await?;

        Ok(())
    }

    async fn on_mint_drop(&self, key: SolanaNftEventKey, tx: SolanaSignedTxn) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::SolanaMintDropSigned(tx)),
        };

        self.producer.send(Some(&event), Some(&key.into())).await?;

        Ok(())
    }

    async fn on_retry_create_drop(
        &self,
        key: SolanaNftEventKey,
        tx: SolanaSignedTxn,
    ) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::SolanaRetryCreateDropSigned(tx)),
        };

        self.producer.send(Some(&event), Some(&key.into())).await?;

        Ok(())
    }

    async fn on_retry_mint_drop(&self, key: SolanaNftEventKey, tx: SolanaSignedTxn) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::SolanaRetryMintDropSigned(tx)),
        };

        self.producer.send(Some(&event), Some(&key.into())).await?;

        Ok(())
    }

    async fn on_update_drop(&self, key: SolanaNftEventKey, tx: SolanaSignedTxn) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::SolanaUpdateDropSigned(tx)),
        };

        self.producer.send(Some(&event), Some(&key.into())).await?;

        Ok(())
    }

    async fn on_transfer_asset(&self, key: SolanaNftEventKey, tx: SolanaSignedTxn) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::SolanaTransferAssetSigned(tx)),
        };

        self.producer.send(Some(&event), Some(&key.into())).await?;

        Ok(())
    }
}

impl From<SolanaNftEventKey> for TreasuryEventKey {
    fn from(
        SolanaNftEventKey {
            id,
            user_id,
            project_id,
        }: SolanaNftEventKey,
    ) -> Self {
        Self {
            id,
            user_id,
            project_id: Some(project_id),
        }
    }
}
