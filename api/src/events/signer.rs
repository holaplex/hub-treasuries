use fireblocks::Fireblocks;
use hub_core::{prelude::*, producer::Producer};

use crate::proto::TreasuryEvents;

#[async_trait]
pub trait Sign<TxType, K, P, T> {
    async fn send_transaction(&self, tx_type: TxType, key: K, payload: P) -> Result<T>;
}

#[async_trait]
pub trait Events<K, T> {
    async fn on_create_drop(&self, key: K, tx: T) -> Result<()>;
    async fn on_mint_drop(&self, key: K, tx: T) -> Result<()>;
    async fn on_update_drop(&self, key: K, tx: T) -> Result<()>;
    async fn on_transfer_asset(&self, key: K, tx: T) -> Result<()>;
    async fn on_retry_create_drop(&self, key: K, tx: T) -> Result<()>;
    async fn on_retry_mint_drop(&self, key: K, tx: T) -> Result<()>;
}

#[async_trait]
pub trait Transactions<TxType, K, P, T>: Sign<TxType, K, P, T> + Events<K, T> {
    async fn create_drop(&self, key: K, payload: P) -> Result<T>;
    async fn mint_drop(&self, key: K, payload: P) -> Result<T>;
    async fn update_drop(&self, key: K, payload: P) -> Result<T>;
    async fn transfer_asset(&self, key: K, payload: P) -> Result<T>;
    async fn retry_create_drop(&self, key: K, payload: P) -> Result<T>;
    async fn retry_mint_drop(&self, key: K, payload: P) -> Result<T>;
}

pub struct TransactionSigner {
    pub fireblocks: Fireblocks,
    pub producer: Producer<TreasuryEvents>,
    pub vault_id: String,
}

impl TransactionSigner {
    pub fn new(
        fireblocks: Fireblocks,
        producer: Producer<TreasuryEvents>,
        vault_id: String,
    ) -> Self {
        Self {
            fireblocks,
            producer,
            vault_id,
        }
    }
}
