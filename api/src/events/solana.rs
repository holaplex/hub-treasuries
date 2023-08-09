use fireblocks::Fireblocks;
use hex::FromHex;
use hub_core::{bs58, futures_util::future, prelude::*, producer::Producer};
use solana_sdk::pubkey::Pubkey;

use super::{
    signer::{find_vault_id_by_wallet_address, sign_message, Sign},
    Result,
};
use crate::{
    db::Connection,
    entities::sea_orm_active_enums::TxType,
    proto::{
        solana_nft_events::Event as SolanaNftEvent,
        treasury_events::{Event, SolanaTransactionResult, TransactionStatus},
        SolanaNftEventKey, SolanaNftEvents, SolanaPendingTransaction, TreasuryEventKey,
        TreasuryEvents,
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

    #[must_use]
    pub fn is_public_key(test_case: &str) -> bool {
        Pubkey::from_str(test_case).is_ok()
    }

    pub async fn process(&self, key: SolanaNftEventKey, e: SolanaNftEvents) -> Result<()> {
        match e.event {
            Some(SolanaNftEvent::CreateDropSigningRequested(payload)) => {
                self.send_and_notify(
                    TxType::CreateDrop,
                    key,
                    payload,
                    Event::SolanaCreateDropSigned,
                )
                .await?;
            },
            Some(SolanaNftEvent::UpdateDropSigningRequested(payload)) => {
                self.send_and_notify(
                    TxType::UpdateMetadata,
                    key,
                    payload,
                    Event::SolanaUpdateDropSigned,
                )
                .await?;
            },
            Some(SolanaNftEvent::MintDropSigningRequested(payload)) => {
                self.send_and_notify(
                    TxType::MintEdition,
                    key,
                    payload,
                    Event::SolanaMintDropSigned,
                )
                .await?;
            },
            Some(SolanaNftEvent::TransferAssetSigningRequested(payload)) => {
                self.send_and_notify(
                    TxType::TransferMint,
                    key,
                    payload,
                    Event::SolanaTransferAssetSigned,
                )
                .await?;
            },
            Some(SolanaNftEvent::RetryCreateDropSigningRequested(payload)) => {
                self.send_and_notify(
                    TxType::CreateDrop,
                    key,
                    payload,
                    Event::SolanaRetryCreateDropSigned,
                )
                .await?;
            },
            Some(SolanaNftEvent::RetryMintDropSigningRequested(payload)) => {
                self.send_and_notify(
                    TxType::MintEdition,
                    key,
                    payload,
                    Event::SolanaRetryMintDropSigned,
                )
                .await?;
            },
            Some(SolanaNftEvent::CreateCollectionSigningRequested(payload)) => {
                self.send_and_notify(
                    TxType::CreateCollection,
                    key,
                    payload,
                    Event::SolanaCreateCollectionSigned,
                )
                .await?;
            },
            Some(SolanaNftEvent::UpdateCollectionSigningRequested(payload)) => {
                self.send_and_notify(
                    TxType::UpdateMetadata,
                    key,
                    payload,
                    Event::SolanaUpdateCollectionSigned,
                )
                .await?;
            },
            Some(SolanaNftEvent::RetryCreateCollectionSigningRequested(payload)) => {
                self.send_and_notify(
                    TxType::CreateCollection,
                    key,
                    payload,
                    Event::SolanaRetryCreateCollectionSigned,
                )
                .await?;
            },
            Some(SolanaNftEvent::MintToCollectionSigningRequested(payload)) => {
                self.send_and_notify(
                    TxType::MintToCollection,
                    key,
                    payload,
                    Event::SolanaMintToCollectionSigned,
                )
                .await?;
            },
            Some(SolanaNftEvent::RetryMintToCollectionSigningRequested(payload)) => {
                self.send_and_notify(
                    TxType::MintToCollection,
                    key,
                    payload,
                    Event::SolanaRetryMintToCollectionSigned,
                )
                .await?;
            },
            _ => (),
        }

        Ok(())
    }
}

#[async_trait]
impl Sign for Solana {
    type Key = SolanaNftEventKey;
    type Payload = SolanaPendingTransaction;
    type Signature = String;
    type Transaction = SolanaTransactionResult;

    const ASSET_ID: &'static str = "SOL";

    #[inline]
    fn producer(&self) -> &Producer<TreasuryEvents> {
        &self.producer
    }

    async fn sign_message(
        &self,
        note: String,
        message: Vec<u8>,
        vault_id: String,
    ) -> Result<String> {
        let sig = sign_message::<Self>(&self.fireblocks, note, message, vault_id).await?;
        let sig = <[u8; 64]>::from_hex(sig.full_sig)?;
        let sig = bs58::encode(sig).into_string();

        Ok(sig)
    }

    async fn send_transaction(
        &self,
        tx_type: TxType,
        key: SolanaNftEventKey,
        SolanaPendingTransaction {
            serialized_message,
            signatures_or_signers_public_keys,
        }: SolanaPendingTransaction,
    ) -> Result<SolanaTransactionResult> {
        let conn = self.db.get();
        let note = format!(
            "{:?} by {:?} for project {:?}",
            tx_type, key.user_id, key.project_id,
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
