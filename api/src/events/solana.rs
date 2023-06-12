use fireblocks::Fireblocks;
use futures::future::try_join_all;
use hex::FromHex;
use hub_core::{anyhow::Error, prelude::*, producer::Producer};

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
    async fn send_transaction(
        &self,
        tx_type: TxType,
        key: SolanaNftEventKey,
        mut payload: SolanaPendingTransaction,
    ) -> Result<SolanaSignedTxn> {
        let conn = self.db.get();
        let note = format!(
            "{:?} by {:?} for project {:?}",
            tx_type, key.user_id, key.project_id,
        );

        info!("looking up vault ids for {:?}", payload.request_signatures.clone());
        let vault_ids =
            Self::find_vault_ids_by_wallet_address(conn, payload.request_signatures.clone()).await?;

        info!("vault ids {:?}", vault_ids.clone());
        let asset_id = self.fireblocks.assets().id("SOL");
        let create_client = self.fireblocks.client().create();

        let transactions = vault_ids.into_iter().map(|vault_id| {
            let asset_id = asset_id.clone();
            let note = note.clone();
            let message = payload.serialized_message.clone();

            info!("sending transaction to fireblocks {vault_id}");
            create_client.raw_transaction(asset_id, vault_id, message, note)
        });
        let transactions = try_join_all(transactions).await?;

        let transaction_details = transactions.into_iter().map(|transaction| {
            info!("waiting on transaction {:?}", transaction);

            self.fireblocks
                .client()
                .wait_on_transaction_completion(transaction.id)
        });

        let transaction_details = try_join_all(transaction_details).await?;

        let signatures = transaction_details
            .iter()
            .map(|transaction_detail| {
                info!("getting signature for transaction {:?}", transaction_detail);

                let full_sig = transaction_detail
                    .signed_messages
                    .get(0)
                    .context("failed to get signed message response")?
                    .clone()
                    .signature
                    .full_sig;

                let signature = <[u8; 64]>::from_hex(full_sig)?;

                let signature = bs58::encode(signature).into_string();

                Ok(signature)
            })
            .collect::<Result<Vec<String>, Error>>()?;

        for signature in signatures {
            payload.signed_message_signatures.push(signature);
        }

        info!("transaction signed successfully {:?}", payload);

        Ok(SolanaSignedTxn {
            serialized_message: payload.serialized_message,
            signed_message_signatures: payload.signed_message_signatures,
        })
    }
}

#[async_trait]
impl Events<SolanaNftEventKey, SolanaSignedTxn> for Solana {
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
