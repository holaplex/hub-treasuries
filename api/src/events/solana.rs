use std::time::Instant;

use hex::FromHex;
use hub_core::{bs58, futures_util::future, metrics::KeyValue, prelude::*, producer::Producer};
use solana_sdk::pubkey::Pubkey;

use super::{
    signer::{find_vault_id_by_wallet_address, sign_message, Sign},
    Processor, Result,
};
use crate::proto::{
    solana_nft_events::Event as SolanaNftEvent,
    treasury_events::{Event, SolanaTransactionResult, TransactionStatus},
    SolanaNftEventKey, SolanaNftEvents, SolanaPendingTransaction, TreasuryEventKey, TreasuryEvents,
};

#[derive(Debug, Clone, Copy)]
pub enum EventKind {
    CreateDrop,
    RetryCreateDrop,
    UpdateDrop,
    MintDrop,
    RetryMintDrop,
    TransferAsset,
    CreateCollection,
    RetryCreateCollection,
    UpdateCollection,
    UpdateCollectionMint,
    RetryUpdateCollectionMint,
    MintToCollection,
    RetryMintToCollection,
    SwitchCollection,
}

impl super::signer::EventKind<SolanaTransactionResult> for EventKind {
    fn to_event(&self, txn: SolanaTransactionResult) -> Event {
        match self {
            EventKind::CreateDrop => Event::SolanaCreateDropSigned(txn),
            EventKind::RetryCreateDrop => Event::SolanaRetryCreateDropSigned(txn),
            EventKind::UpdateDrop => Event::SolanaUpdateDropSigned(txn),
            EventKind::MintDrop => Event::SolanaMintDropSigned(txn),
            EventKind::RetryMintDrop => Event::SolanaRetryMintDropSigned(txn),
            EventKind::TransferAsset => Event::SolanaTransferAssetSigned(txn),
            EventKind::CreateCollection => Event::SolanaCreateCollectionSigned(txn),
            EventKind::RetryCreateCollection => Event::SolanaRetryCreateCollectionSigned(txn),
            EventKind::UpdateCollection => Event::SolanaUpdateCollectionSigned(txn),
            EventKind::UpdateCollectionMint => Event::SolanaUpdateCollectionMintSigned(txn),
            EventKind::RetryUpdateCollectionMint => {
                Event::SolanaRetryUpdateCollectionMintSigned(txn)
            },
            EventKind::MintToCollection => Event::SolanaMintToCollectionSigned(txn),
            EventKind::RetryMintToCollection => Event::SolanaRetryMintToCollectionSigned(txn),
            EventKind::SwitchCollection => Event::SolanaSwitchMintCollectionSigned(txn),
        }
    }
}

pub struct Solana<'a>(&'a Processor);

impl<'a> Solana<'a> {
    #[inline]
    #[must_use]
    pub fn new(processor: &'a Processor) -> Self {
        Self(processor)
    }

    #[must_use]
    pub fn is_public_key(test_case: &str) -> bool {
        Pubkey::from_str(test_case).is_ok()
    }

    pub async fn process(&self, key: SolanaNftEventKey, e: SolanaNftEvents) -> Result<()> {
        match e.event {
            Some(SolanaNftEvent::CreateDropSigningRequested(payload)) => {
                self.send_and_notify(EventKind::CreateDrop, key, payload)
                    .await?;
            },
            Some(SolanaNftEvent::UpdateDropSigningRequested(payload)) => {
                self.send_and_notify(EventKind::UpdateDrop, key, payload)
                    .await?;
            },
            Some(SolanaNftEvent::MintDropSigningRequested(payload)) => {
                self.send_and_notify(EventKind::MintDrop, key, payload)
                    .await?;
            },
            Some(SolanaNftEvent::TransferAssetSigningRequested(payload)) => {
                self.send_and_notify(EventKind::TransferAsset, key, payload)
                    .await?;
            },
            Some(SolanaNftEvent::RetryCreateDropSigningRequested(payload)) => {
                self.send_and_notify(EventKind::RetryCreateDrop, key, payload)
                    .await?;
            },
            Some(SolanaNftEvent::RetryMintDropSigningRequested(payload)) => {
                self.send_and_notify(EventKind::RetryMintDrop, key, payload)
                    .await?;
            },
            Some(SolanaNftEvent::CreateCollectionSigningRequested(payload)) => {
                self.send_and_notify(EventKind::CreateCollection, key, payload)
                    .await?;
            },
            Some(SolanaNftEvent::UpdateCollectionSigningRequested(payload)) => {
                self.send_and_notify(EventKind::UpdateCollection, key, payload)
                    .await?;
            },
            Some(SolanaNftEvent::UpdateCollectionMintSigningRequested(payload)) => {
                self.send_and_notify(EventKind::UpdateCollectionMint, key, payload)
                    .await?;
            },
            Some(SolanaNftEvent::RetryUpdateMintSigningRequested(payload)) => {
                self.send_and_notify(EventKind::RetryUpdateCollectionMint, key, payload)
                    .await?;
            },
            Some(SolanaNftEvent::RetryCreateCollectionSigningRequested(payload)) => {
                self.send_and_notify(EventKind::RetryCreateCollection, key, payload)
                    .await?;
            },
            Some(SolanaNftEvent::MintToCollectionSigningRequested(payload)) => {
                self.send_and_notify(EventKind::MintToCollection, key, payload)
                    .await?;
            },
            Some(SolanaNftEvent::RetryMintToCollectionSigningRequested(payload)) => {
                self.send_and_notify(EventKind::RetryMintToCollection, key, payload)
                    .await?;
            },
            Some(SolanaNftEvent::SwitchMintCollectionSigningRequested(payload)) => {
                self.send_and_notify(EventKind::SwitchCollection, key, payload)
                    .await?;
            },
            _ => (),
        }

        Ok(())
    }
}

#[async_trait]
impl<'a> Sign for Solana<'a> {
    type EventKind = EventKind;
    type Key = SolanaNftEventKey;
    type Payload = SolanaPendingTransaction;
    type Signature = String;
    type Transaction = SolanaTransactionResult;

    const ASSET_ID: &'static str = "SOL";

    #[inline]
    fn producer(&self) -> &Producer<TreasuryEvents> {
        &self.0.producer
    }

    async fn sign_message(
        &self,
        note: String,
        message: Vec<u8>,
        vault_id: String,
    ) -> Result<String> {
        let start = Instant::now();

        let sig = sign_message::<Self>(&self.0.fireblocks, note, message, vault_id).await?;
        let sig = <[u8; 64]>::from_hex(sig.full_sig)?;
        let sig = bs58::encode(sig).into_string();

        let elapsed = i64::try_from(start.elapsed().as_millis()).unwrap_or(0);
        self.0
            .metrics
            .sign_duration_ms_bucket
            .record(elapsed, &[KeyValue::new("blockchain", "Solana")]);

        Ok(sig)
    }

    async fn send_transaction(
        &self,
        kind: EventKind,
        key: SolanaNftEventKey,
        SolanaPendingTransaction {
            serialized_message,
            signatures_or_signers_public_keys,
        }: SolanaPendingTransaction,
    ) -> Result<SolanaTransactionResult> {
        let conn = self.0.db.get();
        let note = format!(
            "{kind:?} by {:?} for project {:?}",
            key.user_id, key.project_id,
        );

        let mut fireblocks_requests = Vec::new();

        for req_sig in signatures_or_signers_public_keys {
            if Self::is_public_key(&req_sig) {
                let vault_id = find_vault_id_by_wallet_address(conn, req_sig).await?;

                let fireblocks_request: future::BoxFuture<Result<String>> =
                    Box::pin(self.sign_message(note.clone(), serialized_message.clone(), vault_id));

                fireblocks_requests.push(fireblocks_request);
            } else {
                fireblocks_requests.push(Box::pin(future::ready(Ok(req_sig.to_string()))));
            }
        }

        let solana_transaction_result =
            future::try_join_all(fireblocks_requests).await.map_or_else(
                |_| SolanaTransactionResult {
                    serialized_message: None,
                    signed_message_signatures: vec![],
                    status: TransactionStatus::Failed.into(),
                },
                |signed_message_signatures| SolanaTransactionResult {
                    serialized_message: Some(serialized_message),
                    signed_message_signatures,
                    status: TransactionStatus::Completed.into(),
                },
            );

        Ok(solana_transaction_result)
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
            project_id,
        }
    }
}
