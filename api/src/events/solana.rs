use std::time::Instant;

use hex::FromHex;
use hub_core::{
    bs58, futures_util::future, metrics::KeyValue, prelude::*, producer::Producer,
    util::ValidateAddress,
};

use super::{
    signer::{find_vault_id_by_wallet_address, sign_message, Sign},
    Processor, ProcessorError, Result,
};
use crate::proto::{
    solana_nft_events::Event as SolanaNftEvent,
    treasury_events::{Event, SolanaTransactionResult, TransactionStatus},
    SolanaMintPendingTransactions, SolanaNftEventKey, SolanaNftEvents, SolanaPendingTransaction,
    TreasuryEventKey, TreasuryEvents,
};

#[derive(Debug, Clone, Copy)]
pub enum EventKind {
    CreateEditionDrop,
    RetryCreateEditionDrop,
    UpdateEditionDrop,
    MintEditionDrop,
    RetryMintEditionDrop,
    TransferAsset,
    CreateCollection,
    RetryCreateCollection,
    UpdateCollection,
    UpdateCollectionMint,
    RetryUpdateCollectionMint,
    MintToCollection,
    RetryMintToCollection,
    SwitchCollection,
    CreateOpenDrop,
    RetryCreateOpenDrop,
    UpdateOpenDrop,
    MintOpenDrop,
    RetryMintOpenDrop,
}

impl super::signer::EventKind<SolanaTransactionResult> for EventKind {
    fn to_event(&self, txn: SolanaTransactionResult) -> Event {
        match self {
            EventKind::CreateEditionDrop => Event::SolanaCreateEditionDropSigned(txn),
            EventKind::RetryCreateEditionDrop => Event::SolanaRetryCreateEditionDropSigned(txn),
            EventKind::UpdateEditionDrop => Event::SolanaUpdateEditionDropSigned(txn),
            EventKind::MintEditionDrop => Event::SolanaMintEditionDropSigned(txn),
            EventKind::RetryMintEditionDrop => Event::SolanaRetryMintEditionDropSigned(txn),
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
            EventKind::CreateOpenDrop => Event::SolanaCreateOpenDropSigned(txn),
            EventKind::RetryCreateOpenDrop => Event::SolanaRetryCreateOpenDropSigned(txn),
            EventKind::UpdateOpenDrop => Event::SolanaUpdateOpenDropSigned(txn),
            EventKind::MintOpenDrop => Event::SolanaMintOpenDropSigned(txn),
            EventKind::RetryMintOpenDrop => Event::SolanaRetryMintOpenDropSigned(txn),
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

    pub async fn process(&self, key: SolanaNftEventKey, e: SolanaNftEvents) -> Result<()> {
        match e.event {
            Some(SolanaNftEvent::CreateEditionDropSigningRequested(payload)) => {
                self.send_and_notify(EventKind::CreateEditionDrop, key, payload)
                    .await?;
            },
            Some(SolanaNftEvent::UpdateEditionDropSigningRequested(payload)) => {
                self.send_and_notify(EventKind::UpdateEditionDrop, key, payload)
                    .await?;
            },
            Some(SolanaNftEvent::MintEditionDropSigningRequested(payload)) => {
                self.send_and_notify(EventKind::MintEditionDrop, key, payload)
                    .await?;
            },
            Some(SolanaNftEvent::TransferAssetSigningRequested(payload)) => {
                self.send_and_notify(EventKind::TransferAsset, key, payload)
                    .await?;
            },
            Some(SolanaNftEvent::RetryCreateEditionDropSigningRequested(payload)) => {
                self.send_and_notify(EventKind::RetryCreateEditionDrop, key, payload)
                    .await?;
            },
            Some(SolanaNftEvent::RetryMintEditionDropSigningRequested(payload)) => {
                self.send_and_notify(EventKind::RetryMintEditionDrop, key, payload)
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
            Some(SolanaNftEvent::CreateOpenDropSigningRequested(payload)) => {
                self.send_and_notify(EventKind::CreateOpenDrop, key, payload)
                    .await?;
            },
            Some(SolanaNftEvent::RetryCreateOpenDropSigningRequested(payload)) => {
                self.send_and_notify(EventKind::RetryCreateOpenDrop, key, payload)
                    .await?;
            },
            Some(SolanaNftEvent::UpdateOpenDropSigningRequested(payload)) => {
                self.send_and_notify(EventKind::UpdateOpenDrop, key, payload)
                    .await?;
            },
            Some(SolanaNftEvent::MintOpenDropSigningRequested(payload)) => {
                self.send_and_notify(EventKind::MintOpenDrop, key, payload)
                    .await?;
            },
            Some(SolanaNftEvent::RetryMintOpenDropSigningRequested(payload)) => {
                self.send_and_notify(EventKind::RetryMintOpenDrop, key, payload)
                    .await?;
            },
            Some(SolanaNftEvent::MintOpenDropBatchedSigningRequested(payload)) => {
                self.sign_mint_batch(key, payload).await?;
            },
            _ => (),
        }

        Ok(())
    }

    pub async fn sign_mint_batch(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaMintPendingTransactions,
    ) -> Result<()> {
        let conn = self.0.db.get();
        let fireblocks = &self.0.fireblocks;
        let pubkeys = payload.signers_pubkeys.clone();

        if pubkeys.len() != 2 {
            return Err(ProcessorError::InvalidNumberOfSigners);
        }

        let note = &format!(
            "Mint batch signing for collection {:?} by {:?} for project {:?}",
            key.id, key.user_id, key.project_id,
        );

        let messages = &payload
            .mint_transactions
            .clone()
            .into_iter()
            .map(|m| m.serialized_message)
            .collect::<Vec<_>>();

        let tx = |vault: String| async move {
            let asset_id = fireblocks.assets().id("SOL");

            let transaction = fireblocks
                .client()
                .create()
                .raw_transaction(asset_id, vault, messages.clone(), note.to_string())
                .await
                .map_err(ProcessorError::Fireblocks)?;

            let details = fireblocks
                .client()
                .wait_on_transaction_completion(transaction.id)
                .await
                .map_err(ProcessorError::Fireblocks)?;

            Result::<_>::Ok(details)
        };

        let mut futures = Vec::new();

        for req_sig in pubkeys.clone() {
            let vault_id = find_vault_id_by_wallet_address(conn, req_sig).await?;
            futures.push(tx(vault_id));
        }

        let futs_result = future::join_all(futures)
            .await
            .into_iter()
            .map(|r| r.map(|d| d.signed_messages))
            .collect::<Result<Vec<_>>>()?;

        let signatures = futs_result[0]
            .clone()
            .into_iter()
            .zip(futs_result[1].clone().into_iter())
            .zip(
                payload
                    .mint_transactions
                    .iter()
                    .map(|m| m.signer_signature.clone()),
            )
            .collect::<Vec<_>>();

        for ((sig1, sig2), sig3) in signatures {
            let key = key.clone();
            let mut signatures = Vec::new();
            let content = bs58::decode(sig1.content).into_vec()?;

            let sig1_bytes = <[u8; 64]>::from_hex(sig1.signature.full_sig)?;
            signatures.push(bs58::encode(sig1_bytes).into_string());

            let sig2_bytes = <[u8; 64]>::from_hex(sig2.signature.full_sig)?;
            signatures.push(bs58::encode(sig2_bytes).into_string());

            // Uncompressed mint message needs to be signed by mint key pair
            if let Some(sig3) = sig3 {
                signatures.insert(1, sig3);
            }

            let txn = SolanaTransactionResult {
                serialized_message: Some(content),
                signed_message_signatures: signatures,
                status: TransactionStatus::Completed.into(),
            };

            let evt = Event::SolanaMintOpenDropSigned(txn);
            self.producer()
                .send(
                    Some(&TreasuryEvents { event: Some(evt) }),
                    Some(&key.into()),
                )
                .await?;
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
            if ValidateAddress::is_solana_address(&req_sig) {
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
