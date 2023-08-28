use fireblocks::{objects::transaction::SignatureResponse, Fireblocks};
use hub_core::{prelude::*, producer::Producer};
use sea_orm::{prelude::*, DatabaseConnection, JoinType, QueryFilter, QuerySelect, RelationTrait};

use super::{ProcessorError, Result};
use crate::{
    entities::{treasuries, wallets},
    proto::{treasury_events::Event, TreasuryEventKey, TreasuryEvents},
};

pub trait EventKind<T>: Copy {
    fn to_event(&self, txn: T) -> Event;
}

#[async_trait]
pub trait Sign {
    type EventKind: fmt::Debug + Send + EventKind<Self::Transaction>;
    type Signature;
    type Key: Clone + Into<TreasuryEventKey> + Send;
    type Payload: Send;
    type Transaction: Clone + Send;

    const ASSET_ID: &'static str;

    fn producer(&self) -> &Producer<TreasuryEvents>;

    async fn sign_message(
        &self,
        note: String,
        message: Vec<u8>,
        vault_id: String,
    ) -> Result<Self::Signature>;

    async fn send_and_notify(
        &self,
        kind: Self::EventKind,
        key: Self::Key,
        txn: Self::Payload,
    ) -> Result<Self::Transaction> {
        let k = key.clone();
        let txn = self.send_transaction(kind, k, txn).await?;

        let evt = kind.to_event(txn.clone());
        self.producer()
            .send(
                Some(&TreasuryEvents { event: Some(evt) }),
                Some(&key.into()),
            )
            .await?;

        Ok(txn)
    }

    async fn send_transaction(
        &self,
        kind: Self::EventKind,
        key: Self::Key,
        payload: Self::Payload,
    ) -> Result<Self::Transaction>;
}

pub(crate) async fn sign_message<G: Sign>(
    fireblocks: &Fireblocks,
    note: String,
    message: Vec<u8>,
    vault_id: String,
) -> Result<SignatureResponse> {
    let asset_id = fireblocks.assets().id(G::ASSET_ID);

    let transaction = fireblocks
        .client()
        .create()
        .raw_transaction(asset_id, vault_id, message, note)
        .await
        .map_err(ProcessorError::Fireblocks)?;

    let details = fireblocks
        .client()
        .wait_on_transaction_completion(transaction.id)
        .await
        .map_err(ProcessorError::Fireblocks)?;

    Ok(details
        .signed_messages
        .get(0)
        .ok_or(ProcessorError::MissingSignedMessage)?
        .clone()
        .signature)
}

pub(crate) async fn find_vault_id_by_wallet_address(
    db: &DatabaseConnection,
    wallet_address: String,
) -> Result<String> {
    let treasury = treasuries::Entity::find()
        .join(JoinType::InnerJoin, treasuries::Relation::Wallets.def())
        .filter(wallets::Column::Address.eq(wallet_address.clone()))
        .one(db)
        .await?
        .ok_or_else(|| ProcessorError::InvalidWalletAddress(wallet_address))?;

    Ok(treasury.vault_id)
}
