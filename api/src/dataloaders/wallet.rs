use std::collections::HashMap;

use async_graphql::{dataloader::Loader as DataLoader, FieldError, Result};
use poem::async_trait;
use sea_orm::{prelude::*, JoinType, QuerySelect};

use crate::{
    db::Connection,
    entities::{customer_treasuries, treasuries, wallets},
};

///  A struct that implements a `DataLoader` for loading wallet models by their UUID.
#[derive(Debug, Clone)]
pub struct WalletLoader {
    pub db: Connection,
}

impl WalletLoader {
    #[must_use]
    pub fn new(db: Connection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl DataLoader<String> for WalletLoader {
    type Error = FieldError;
    type Value = wallets::Model;

    async fn load(&self, keys: &[String]) -> Result<HashMap<String, Self::Value>, Self::Error> {
        let wallets = wallets::Entity::find()
            .filter(wallets::Column::Address.is_in(keys.iter().map(ToOwned::to_owned)))
            .all(self.db.get())
            .await?;

        Ok(wallets
            .into_iter()
            .map(|i| {
                let address = i.address.clone().ok_or_else(|| {
                    Self::Error::new(format!("Address is missing for wallet with ID {}", i.id))
                })?;
                Ok((address, i))
            })
            .collect::<Result<_>>()?)
    }
}

///  A struct that implements a `DataLoader` for loading wallet models associated with treasury models by their UUID.
#[derive(Debug, Clone)]
pub struct TreasuryWalletsLoader {
    pub db: Connection,
}

impl TreasuryWalletsLoader {
    #[must_use]
    pub fn new(db: Connection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl DataLoader<Uuid> for TreasuryWalletsLoader {
    type Error = FieldError;
    type Value = Vec<wallets::Model>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let treasuries = treasuries::Entity::find()
            .find_with_related(wallets::Entity)
            .filter(treasuries::Column::Id.is_in(keys.iter().map(ToOwned::to_owned)))
            .all(self.db.get())
            .await?;

        Ok(treasuries
            .into_iter()
            .map(|(treasury, wallets)| (treasury.id, wallets))
            .collect())
    }
}

/// A struct that implements a `DataLoader` for loading wallet models associated with customer treasury models by their UUID.
#[derive(Debug, Clone)]
pub struct CustomerTreasuryWalletLoader {
    pub db: Connection,
}

impl CustomerTreasuryWalletLoader {
    #[must_use]
    pub fn new(db: Connection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl DataLoader<Uuid> for CustomerTreasuryWalletLoader {
    type Error = FieldError;
    type Value = Vec<wallets::Model>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let wallets = customer_treasuries::Entity::find()
            .join(
                JoinType::InnerJoin,
                customer_treasuries::Entity::belongs_to(wallets::Entity)
                    .from(customer_treasuries::Column::TreasuryId)
                    .to(wallets::Column::TreasuryId)
                    .into(),
            )
            .select_with(wallets::Entity)
            .filter(
                customer_treasuries::Column::CustomerId.is_in(keys.iter().map(ToOwned::to_owned)),
            )
            .all(self.db.get())
            .await?;

        Ok(wallets
            .into_iter()
            .map(|(ct, wallets)| (ct.customer_id, wallets))
            .collect())
    }
}
