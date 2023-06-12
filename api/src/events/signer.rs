use hub_core::prelude::*;
use sea_orm::{prelude::*, DatabaseConnection, JoinType, QueryFilter, QuerySelect, RelationTrait};

use crate::entities::{sea_orm_active_enums::TxType, treasuries, wallets};

#[async_trait]
pub trait Sign<K, P, T> {
    async fn send_transaction(&self, tx_type: TxType, key: K, payload: P) -> Result<T>;
    async fn find_vault_ids_by_wallet_address(
        db: &DatabaseConnection,
        wallet_addresses: Vec<String>,
    ) -> Result<Vec<String>> {
        let treasuries = treasuries::Entity::find()
            .join(JoinType::InnerJoin, treasuries::Relation::Wallets.def())
            .filter(wallets::Column::Address.is_in(wallet_addresses))
            .all(db)
            .await?;

        info!("found treasury vault ids: {:?}", treasuries);

        Ok(treasuries.into_iter().map(|t| t.vault_id).collect())
    }
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
