use fireblocks::Fireblocks;
use hub_core::{prelude::*, producer::Producer};

use super::signer::{Events, Sign, TransactionSigner, Transactions};
use crate::{
    entities::sea_orm_active_enums::TxType,
    proto::{
        treasury_events::{Event, PolygonTxnResult, TransactionStatus},
        PolygonNftEventKey, PolygonTransaction, TreasuryEventKey, TreasuryEvents,
    },
};

pub struct Polygon {
    fireblocks: Fireblocks,
    producer: Producer<TreasuryEvents>,
}

impl Polygon {
    #[must_use]
    pub fn new(fireblocks: Fireblocks, producer: Producer<TreasuryEvents>) -> Self {
        Self {
            fireblocks,
            producer,
        }
    }

    #[must_use]
    pub fn signer(&self, vault_id: String) -> TransactionSigner {
        TransactionSigner::new(self.fireblocks.clone(), self.producer.clone(), vault_id)
    }
}

#[async_trait]
impl Transactions<TxType, PolygonNftEventKey, PolygonTransaction, PolygonTxnResult>
    for TransactionSigner
{
    async fn create_drop(
        &self,
        key: PolygonNftEventKey,
        payload: PolygonTransaction,
    ) -> Result<PolygonTxnResult> {
        self.send_transaction(TxType::CreateDrop, key, payload)
            .await
    }

    async fn update_drop(
        &self,
        key: PolygonNftEventKey,
        payload: PolygonTransaction,
    ) -> Result<PolygonTxnResult> {
        self.send_transaction(TxType::UpdateMetadata, key, payload)
            .await
    }

    async fn mint_drop(
        &self,
        key: PolygonNftEventKey,
        payload: PolygonTransaction,
    ) -> Result<PolygonTxnResult> {
        self.send_transaction(TxType::MintEdition, key, payload)
            .await
    }

    async fn transfer_asset(
        &self,
        key: PolygonNftEventKey,
        payload: PolygonTransaction,
    ) -> Result<PolygonTxnResult> {
        self.send_transaction(TxType::TransferMint, key, payload)
            .await
    }

    async fn retry_create_drop(
        &self,
        key: PolygonNftEventKey,
        payload: PolygonTransaction,
    ) -> Result<PolygonTxnResult> {
        self.send_transaction(TxType::CreateDrop, key, payload)
            .await
    }

    async fn retry_mint_drop(
        &self,
        key: PolygonNftEventKey,
        payload: PolygonTransaction,
    ) -> Result<PolygonTxnResult> {
        self.send_transaction(TxType::MintEdition, key, payload)
            .await
    }
}

#[async_trait]
impl Sign<TxType, PolygonNftEventKey, PolygonTransaction, PolygonTxnResult> for TransactionSigner {
    async fn send_transaction(
        &self,
        tx_type: TxType,
        key: PolygonNftEventKey,
        payload: PolygonTransaction,
    ) -> Result<PolygonTxnResult> {
        let note = format!(
            "{:?} by {:?} for project {:?}",
            tx_type, key.user_id, key.project_id
        );

        debug!("Submitting Polygon transaction: {:?}", note);
        let vault = self.fireblocks.treasury_vault();

        let transaction = self
            .fireblocks
            .client()
            .create()
            .contract_call(payload.data, "MATIC", vault, note)
            .await?;

        debug!("transaction {:?}", transaction);

        let details = self
            .fireblocks
            .client()
            .wait_on_transaction_completion(transaction.id)
            .await;

        Ok(match details {
            Ok(details) => PolygonTxnResult {
                hash: Some(details.tx_hash),
                status: details.status as i32,
            },
            Err(_) => PolygonTxnResult {
                hash: None,
                status: TransactionStatus::Failed as i32,
            },
        })
    }
}

#[async_trait]
impl Events<PolygonNftEventKey, PolygonTxnResult> for TransactionSigner {
    async fn on_create_drop(&self, key: PolygonNftEventKey, tx: PolygonTxnResult) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::PolygonCreateDropTxnSubmitted(tx)),
        };

        self.producer.send(Some(&event), Some(&key.into())).await?;

        Ok(())
    }

    async fn on_mint_drop(&self, _key: PolygonNftEventKey, _tx: PolygonTxnResult) -> Result<()> {
        Ok(())
    }

    async fn on_retry_create_drop(
        &self,
        _key: PolygonNftEventKey,
        _tx: PolygonTxnResult,
    ) -> Result<()> {
        Ok(())
    }

    async fn on_retry_mint_drop(
        &self,
        _key: PolygonNftEventKey,
        _tx: PolygonTxnResult,
    ) -> Result<()> {
        Ok(())
    }

    async fn on_update_drop(&self, _key: PolygonNftEventKey, _tx: PolygonTxnResult) -> Result<()> {
        Ok(())
    }

    async fn on_transfer_asset(
        &self,
        _key: PolygonNftEventKey,
        _tx: PolygonTxnResult,
    ) -> Result<()> {
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
            project_id: Some(project_id),
        }
    }
}
