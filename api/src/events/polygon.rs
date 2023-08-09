use fireblocks::{objects::transaction::SignatureResponse, Fireblocks};
use hub_core::{prelude::*, producer::Producer};

use super::{
    signer::{find_vault_id_by_wallet_address, sign_message, Sign},
    EcdsaSignatureScalar, ProcessorError, Result,
};
use crate::{
    db::Connection,
    entities::sea_orm_active_enums::TxType,
    proto::{
        polygon_nft_events::Event as PolygonNftEvent,
        treasury_events::{
            EcdsaSignature, Event, PolygonPermitHashSignature, PolygonTransactionResult,
            TransactionStatus,
        },
        PermitArgsHash, PolygonNftEventKey, PolygonNftEvents, PolygonTokenTransferTxns,
        PolygonTransaction, TreasuryEventKey, TreasuryEvents,
    },
};

pub struct Polygon {
    fireblocks: Fireblocks,
    producer: Producer<TreasuryEvents>,
    db: Connection,
}

impl Polygon {
    #[must_use]
    pub fn new(fireblocks: Fireblocks, producer: Producer<TreasuryEvents>, db: Connection) -> Self {
        Self {
            fireblocks,
            producer,
            db,
        }
    }

    pub async fn process(&self, key: PolygonNftEventKey, e: PolygonNftEvents) -> Result<()> {
        match e.event {
            Some(PolygonNftEvent::SubmitCreateDropTxn(payload)) => {
                self.send_and_notify(
                    TxType::CreateDrop,
                    key,
                    payload,
                    Event::PolygonCreateDropTxnSubmitted,
                )
                .await?;
            },
            Some(PolygonNftEvent::SubmitRetryCreateDropTxn(payload)) => {
                self.send_and_notify(
                    TxType::CreateDrop,
                    key,
                    payload,
                    Event::PolygonRetryCreateDropSubmitted,
                )
                .await?;
            },
            Some(PolygonNftEvent::SubmitMintDropTxn(payload)) => {
                self.send_and_notify(
                    TxType::MintEdition,
                    key,
                    payload,
                    Event::PolygonMintDropSubmitted,
                )
                .await?;
            },
            Some(PolygonNftEvent::SubmitUpdateDropTxn(payload)) => {
                self.send_and_notify(
                    TxType::UpdateMetadata,
                    key,
                    payload,
                    Event::PolygonUpdateDropSubmitted,
                )
                .await?;
            },

            Some(PolygonNftEvent::SubmitRetryMintDropTxn(payload)) => {
                self.send_and_notify(
                    TxType::MintEdition,
                    key,
                    payload,
                    Event::PolygonRetryMintDropSubmitted,
                )
                .await?;
            },
            Some(PolygonNftEvent::SignPermitTokenTransferHash(payload)) => {
                self.sign_permit_token_transfer_hash(key, payload).await?;
            },
            Some(PolygonNftEvent::SubmitTransferAssetTxns(payload)) => {
                self.submit_transfer_asset_txns(key, payload).await?;
            },
            Some(PolygonNftEvent::UpdateMintsOwner(_)) | None => (),
        }

        Ok(())
    }

    async fn sign_permit_token_transfer_hash(
        &self,
        key: PolygonNftEventKey,
        payload: PermitArgsHash,
    ) -> Result<()> {
        let PermitArgsHash {
            data,
            owner,
            spender,
            recipient,
            edition_id,
            amount,
        } = payload;

        let vault_id = find_vault_id_by_wallet_address(self.db.get(), owner.clone()).await?;
        let signature = self.sign_message(String::new(), data, vault_id).await?;

        let (r, s, v) = (
            hex::decode(signature.r.ok_or(ProcessorError::IncompleteEcdsaSignature(
                EcdsaSignatureScalar::R,
            ))?)?,
            hex::decode(signature.s.ok_or(ProcessorError::IncompleteEcdsaSignature(
                EcdsaSignatureScalar::S,
            ))?)?,
            (signature.v.ok_or(ProcessorError::IncompleteEcdsaSignature(
                EcdsaSignatureScalar::V,
            ))? + 27)
                .try_into()
                .map_err(ProcessorError::InvalidEcdsaPubkeyRecovery)?,
        );

        let event = TreasuryEvents {
            event: Some(Event::PolygonPermitTransferTokenHashSigned(
                PolygonPermitHashSignature {
                    signature: Some(EcdsaSignature { r, s, v }),
                    owner,
                    spender,
                    recipient,
                    edition_id,
                    amount,
                },
            )),
        };

        self.producer
            .send(Some(&event), Some(&key.into()))
            .await
            .map_err(Into::into)
    }

    async fn submit_transfer_asset_txns(
        &self,
        key: PolygonNftEventKey,
        payload: PolygonTokenTransferTxns,
    ) -> Result<PolygonTransactionResult> {
        let PolygonTokenTransferTxns {
            permit_token_transfer_txn,
            safe_transfer_from_txn,
        } = payload;
        let permit_txn_data =
            permit_token_transfer_txn.ok_or(ProcessorError::MissingPermitTokenTransferTxn)?;
        let safe_txn_data =
            safe_transfer_from_txn.ok_or(ProcessorError::MissingSafeTransferFromTxn)?;

        self.send_transaction(TxType::TransferMint, key.clone(), permit_txn_data)
            .await?;

        self.send_and_notify(
            TxType::TransferMint,
            key,
            safe_txn_data,
            Event::PolygonTransferAssetSubmitted,
        )
        .await
    }
}

#[async_trait]
impl Sign for Polygon {
    type Key = PolygonNftEventKey;
    type Payload = PolygonTransaction;
    type Signature = SignatureResponse;
    type Transaction = PolygonTransactionResult;

    const ASSET_ID: &'static str = "MATIC";

    #[inline]
    fn producer(&self) -> &Producer<TreasuryEvents> {
        &self.producer
    }

    async fn sign_message(
        &self,
        note: String,
        message: Vec<u8>,
        vault_id: String,
    ) -> Result<SignatureResponse> {
        sign_message::<Self>(&self.fireblocks, note, message, vault_id).await
    }

    async fn send_transaction(
        &self,
        tx_type: TxType,
        key: PolygonNftEventKey,
        payload: PolygonTransaction,
    ) -> Result<PolygonTransactionResult> {
        let note = format!(
            "{:?} by {:?} for project {:?}",
            tx_type, key.user_id, key.project_id,
        );
        let vault = self.fireblocks.treasury_vault();
        let asset_id = self.fireblocks.assets().id(Self::ASSET_ID);

        let transaction = self
            .fireblocks
            .client()
            .create()
            .contract_call(payload.data, asset_id, vault, note)
            .await
            .map_err(ProcessorError::Fireblocks)?;

        let (hash, status) = self
            .fireblocks
            .client()
            .wait_on_transaction_completion(transaction.id)
            .await
            .map_or_else(
                |_| (None, TransactionStatus::Failed as i32),
                |details| (Some(details.tx_hash), details.status as i32),
            );

        Ok(PolygonTransactionResult {
            hash,
            status,
            contract_address: payload.contract_address,
            edition_id: payload.edition_id,
        })
    }
}

impl From<PolygonNftEventKey> for TreasuryEventKey {
    fn from(
        PolygonNftEventKey {
            id,
            user_id,
            project_id,
        }: PolygonNftEventKey,
    ) -> Self {
        Self {
            id,
            user_id,
            project_id,
        }
    }
}
