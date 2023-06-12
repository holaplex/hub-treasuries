use fireblocks::{objects::transaction::SignatureResponse, Fireblocks};
use hub_core::{prelude::*, producer::Producer};

use super::signer::{find_vault_id_by_wallet_address, Events, Sign, Transactions};
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
                self.create_drop(key.clone(), payload).await?;
            },
            Some(PolygonNftEvent::SubmitRetryCreateDropTxn(payload)) => {
                self.retry_create_drop(key.clone(), payload).await?;
            },
            Some(PolygonNftEvent::SubmitMintDropTxn(payload)) => {
                self.mint_drop(key.clone(), payload).await?;
            },
            Some(PolygonNftEvent::SubmitUpdateDropTxn(payload)) => {
                self.update_drop(key.clone(), payload).await?;
            },

            Some(PolygonNftEvent::SubmitRetryMintDropTxn(payload)) => {
                self.retry_mint_drop(key.clone(), payload).await?;
            },
            Some(PolygonNftEvent::SignPermitTokenTransferHash(PermitArgsHash {
                data,
                owner,
                spender,
                recipient,
                edition_id,
                amount,
            })) => {
                let vault_id =
                    find_vault_id_by_wallet_address(self.db.get(), owner.clone()).await?;
                let signature = self.sign_message(data, vault_id).await?;

                let (r, s, v) = (
                    hex::decode(
                        signature
                            .r
                            .context("r component of ECDA Signature not found")?,
                    )?,
                    hex::decode(
                        signature
                            .s
                            .context("s component of ECDA Signature not found")?,
                    )?,
                    (signature
                        .v
                        .context("v component of ECDA Signature not found")?
                        + 27)
                        .try_into()?,
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

                self.producer.send(Some(&event), Some(&key.into())).await?;
            },
            Some(PolygonNftEvent::SubmitTransferAssetTxns(payload)) => {
                let PolygonTokenTransferTxns {
                    permit_token_transfer_txn,
                    safe_transfer_from_txn,
                } = payload;
                let permit_txn_data =
                    permit_token_transfer_txn.context("permit_token_transfer_txn not found")?;
                let safe_txn_data =
                    safe_transfer_from_txn.context("safe_transfer_from_txn not found")?;

                self.send_transaction(TxType::TransferMint, key.clone(), permit_txn_data)
                    .await?;

                self.transfer_asset(key, safe_txn_data).await?;
            },
            None => (),
        }

        Ok(())
    }

    pub async fn sign_message(
        &self,
        message: Vec<u8>,
        vault_id: String,
    ) -> Result<SignatureResponse> {
        let asset_id = self.fireblocks.assets().id(Self::ASSET_ID);

        let transaction = self
            .fireblocks
            .client()
            .create()
            .raw_transaction(asset_id, vault_id, message, String::new())
            .await?;

        let details = self
            .fireblocks
            .client()
            .wait_on_transaction_completion(transaction.id)
            .await?;

        let signature = details
            .signed_messages
            .get(0)
            .context("no signed message found")?
            .clone()
            .signature;

        Ok(signature)
    }
}

#[async_trait]
impl Transactions<PolygonNftEventKey, PolygonTransaction, PolygonTransactionResult> for Polygon {
    async fn create_drop(
        &self,
        key: PolygonNftEventKey,
        payload: PolygonTransaction,
    ) -> Result<PolygonTransactionResult> {
        let tx = self
            .send_transaction(TxType::CreateDrop, key.clone(), payload)
            .await?;
        self.on_create_drop(key, tx.clone()).await?;

        Ok(tx)
    }

    async fn update_drop(
        &self,
        key: PolygonNftEventKey,
        payload: PolygonTransaction,
    ) -> Result<PolygonTransactionResult> {
        let tx = self
            .send_transaction(TxType::UpdateMetadata, key.clone(), payload)
            .await?;

        self.on_update_drop(key, tx.clone()).await?;

        Ok(tx)
    }

    async fn mint_drop(
        &self,
        key: PolygonNftEventKey,
        payload: PolygonTransaction,
    ) -> Result<PolygonTransactionResult> {
        let tx = self
            .send_transaction(TxType::MintEdition, key.clone(), payload)
            .await?;

        self.on_mint_drop(key, tx.clone()).await?;

        Ok(tx)
    }

    async fn transfer_asset(
        &self,
        key: PolygonNftEventKey,
        payload: PolygonTransaction,
    ) -> Result<PolygonTransactionResult> {
        let tx = self
            .send_transaction(TxType::TransferMint, key.clone(), payload)
            .await?;

        self.on_transfer_asset(key, tx.clone()).await?;

        Ok(tx)
    }

    async fn retry_create_drop(
        &self,
        key: PolygonNftEventKey,
        payload: PolygonTransaction,
    ) -> Result<PolygonTransactionResult> {
        let tx = self
            .send_transaction(TxType::CreateDrop, key.clone(), payload)
            .await?;
        self.on_retry_create_drop(key, tx.clone()).await?;

        Ok(tx)
    }

    async fn retry_mint_drop(
        &self,
        key: PolygonNftEventKey,
        payload: PolygonTransaction,
    ) -> Result<PolygonTransactionResult> {
        let tx = self
            .send_transaction(TxType::MintEdition, key.clone(), payload)
            .await?;

        self.on_retry_mint_drop(key, tx.clone()).await?;

        Ok(tx)
    }
}

#[async_trait]
impl Sign<PolygonNftEventKey, PolygonTransaction, PolygonTransactionResult> for Polygon {
    const ASSET_ID: &'static str = "MATIC";

    async fn send_transaction(
        &self,
        tx_type: TxType,
        key: PolygonNftEventKey,
        payload: PolygonTransaction,
    ) -> Result<PolygonTransactionResult> {
        let note = format!(
            "{:?} by {:?} for project {:?}",
            tx_type, key.user_id, key.project_id
        );
        let vault = self.fireblocks.treasury_vault();
        let asset_id = self.fireblocks.assets().id(Self::ASSET_ID);

        let transaction = self
            .fireblocks
            .client()
            .create()
            .contract_call(payload.data, asset_id, vault, note)
            .await?;

        let details = self
            .fireblocks
            .client()
            .wait_on_transaction_completion(transaction.id)
            .await;

        Ok(match details {
            Ok(details) => PolygonTransactionResult {
                hash: details.tx_hash,
                status: details.status as i32,
            },
            Err(_) => PolygonTransactionResult {
                hash: String::new(),
                status: TransactionStatus::Failed as i32,
            },
        })
    }
}

#[async_trait]
impl Events<PolygonNftEventKey, PolygonTransactionResult> for Polygon {
    async fn on_create_drop(
        &self,
        key: PolygonNftEventKey,
        tx: PolygonTransactionResult,
    ) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::PolygonCreateDropTxnSubmitted(tx)),
        };

        self.producer.send(Some(&event), Some(&key.into())).await?;

        Ok(())
    }

    async fn on_mint_drop(
        &self,
        key: PolygonNftEventKey,
        tx: PolygonTransactionResult,
    ) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::PolygonMintDropSubmitted(tx)),
        };

        self.producer.send(Some(&event), Some(&key.into())).await?;

        Ok(())
    }

    async fn on_retry_create_drop(
        &self,
        key: PolygonNftEventKey,
        tx: PolygonTransactionResult,
    ) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::PolygonRetryCreateDropSubmitted(tx)),
        };

        self.producer.send(Some(&event), Some(&key.into())).await?;

        Ok(())
    }

    async fn on_retry_mint_drop(
        &self,
        key: PolygonNftEventKey,
        tx: PolygonTransactionResult,
    ) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::PolygonRetryMintDropSubmitted(tx)),
        };

        self.producer.send(Some(&event), Some(&key.into())).await?;

        Ok(())
    }

    async fn on_update_drop(
        &self,
        key: PolygonNftEventKey,
        tx: PolygonTransactionResult,
    ) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::PolygonUpdateDropSubmitted(tx)),
        };

        self.producer.send(Some(&event), Some(&key.into())).await?;

        Ok(())
    }

    async fn on_transfer_asset(
        &self,
        key: PolygonNftEventKey,
        tx: PolygonTransactionResult,
    ) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::PolygonTransferAssetSubmitted(tx)),
        };

        self.producer.send(Some(&event), Some(&key.into())).await?;

        Ok(())
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
