use fireblocks::Fireblocks;
use hub_core::{prelude::*, producer::Producer};

use crate::{entities::sea_orm_active_enums::TxType, proto::TreasuryEvents};

#[async_trait]
pub trait Sign<K, P, T> {
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
pub trait Transactions<K, P, T>: Sign<K, P, T> + Events<K, T> {
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
    pub vault_id: Option<String>,
}

impl TransactionSigner {
    pub fn new(
        fireblocks: Fireblocks,
        producer: Producer<TreasuryEvents>,
        vault_id: Option<String>,
    ) -> Self {
        Self {
            fireblocks,
            producer,
            vault_id,
        }
    }
}
