use fireblocks::Fireblocks;
use hex::FromHex;
use hub_core::{prelude::*, producer::Producer};

use super::signer::{Events, Sign, TransactionSigner, Transactions};
use crate::{
    entities::sea_orm_active_enums::TxType,
    proto::{
        treasury_events::{Event, SolanaSignedTxn},
        SolanaNftEventKey, SolanaTransaction, TreasuryEventKey, TreasuryEvents,
    },
};

pub struct Solana {
    fireblocks: Fireblocks,
    producer: Producer<TreasuryEvents>,
}

impl Solana {
    #[must_use]
    pub fn new(fireblocks: Fireblocks, producer: Producer<TreasuryEvents>) -> Self {
        Self {
            fireblocks,
            producer,
        }
    }

    #[must_use]
    pub fn signer(&self, vault_id: String) -> TransactionSigner {
        TransactionSigner::new(
            self.fireblocks.clone(),
            self.producer.clone(),
            Some(vault_id),
        )
    }
}

#[async_trait]
impl Transactions<SolanaNftEventKey, SolanaTransaction, SolanaSignedTxn> for TransactionSigner {
    async fn create_drop(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaTransaction,
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
        payload: SolanaTransaction,
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
        payload: SolanaTransaction,
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
        payload: SolanaTransaction,
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
        payload: SolanaTransaction,
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
        payload: SolanaTransaction,
    ) -> Result<SolanaSignedTxn> {
        let tx = self
            .send_transaction(TxType::MintEdition, key.clone(), payload)
            .await?;

        self.on_retry_mint_drop(key, tx.clone()).await?;
        Ok(tx)
    }
}

#[async_trait]
impl Sign<SolanaNftEventKey, SolanaTransaction, SolanaSignedTxn> for TransactionSigner {
    async fn send_transaction(
        &self,
        tx_type: TxType,
        key: SolanaNftEventKey,
        mut payload: SolanaTransaction,
    ) -> Result<SolanaSignedTxn> {
        let note = format!(
            "{:?} by {:?} for project {:?}",
            tx_type, key.user_id, key.project_id
        );

        let vault_id = self.vault_id.clone().context("vault id not set")?;
        let asset_id = self.fireblocks.assets().id("SOL");

        let transaction = self
            .fireblocks
            .client()
            .create()
            .raw_transaction(asset_id, vault_id, payload.serialized_message.clone(), note)
            .await?;

        let transaction_details = self
            .fireblocks
            .client()
            .wait_on_transaction_completion(transaction.id)
            .await?;

        let full_sig = transaction_details
            .signed_messages
            .get(0)
            .context("failed to get signed message response")?
            .clone()
            .signature
            .full_sig;

        let signature = <[u8; 64]>::from_hex(full_sig)?;

        payload
            .signed_message_signatures
            .push(bs58::encode(signature).into_string());

        Ok(SolanaSignedTxn {
            serialized_message: payload.serialized_message,
            signed_message_signatures: payload.signed_message_signatures,
        })
    }
}

#[async_trait]
impl Events<SolanaNftEventKey, SolanaSignedTxn> for TransactionSigner {
    async fn on_create_drop(&self, key: SolanaNftEventKey, tx: SolanaSignedTxn) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::SolanaCreateDropSigned(SolanaSignedTxn {
                serialized_message: tx.serialized_message,
                signed_message_signatures: tx.signed_message_signatures,
            })),
        };

        self.producer.send(Some(&event), Some(&key.into())).await?;

        Ok(())
    }

    async fn on_mint_drop(&self, key: SolanaNftEventKey, tx: SolanaSignedTxn) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::SolanaMintDropSigned(SolanaSignedTxn {
                serialized_message: tx.serialized_message,
                signed_message_signatures: tx.signed_message_signatures,
            })),
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
            event: Some(Event::SolanaRetryCreateDropSigned(SolanaSignedTxn {
                serialized_message: tx.serialized_message,
                signed_message_signatures: tx.signed_message_signatures,
            })),
        };

        self.producer.send(Some(&event), Some(&key.into())).await?;

        Ok(())
    }

    async fn on_retry_mint_drop(&self, key: SolanaNftEventKey, tx: SolanaSignedTxn) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::SolanaRetryMintDropSigned(SolanaSignedTxn {
                serialized_message: tx.serialized_message,
                signed_message_signatures: tx.signed_message_signatures,
            })),
        };

        self.producer.send(Some(&event), Some(&key.into())).await?;

        Ok(())
    }

    async fn on_update_drop(&self, key: SolanaNftEventKey, tx: SolanaSignedTxn) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::SolanaUpdateDropSigned(SolanaSignedTxn {
                serialized_message: tx.serialized_message,
                signed_message_signatures: tx.signed_message_signatures,
            })),
        };

        self.producer.send(Some(&event), Some(&key.into())).await?;

        Ok(())
    }

    async fn on_transfer_asset(&self, key: SolanaNftEventKey, tx: SolanaSignedTxn) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::SolanaTransferAssetSigned(SolanaSignedTxn {
                serialized_message: tx.serialized_message,
                signed_message_signatures: tx.signed_message_signatures,
            })),
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
