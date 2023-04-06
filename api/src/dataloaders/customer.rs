use std::collections::HashMap;

use async_graphql::{dataloader::Loader as DataLoader, FieldError, Result};
use poem::async_trait;
use sea_orm::{prelude::*, JoinType, QuerySelect};

use crate::{
    db::Connection,
    entities::{customer_treasuries, wallets},
};

#[derive(Debug, Clone)]
pub struct WalletAddressesLoader {
    pub db: Connection,
}

impl WalletAddressesLoader {
    #[must_use]
    pub fn new(db: Connection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl DataLoader<Uuid> for WalletAddressesLoader {
    type Error = FieldError;
    type Value = Vec<String>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let conn = self.db.get();

        let customer_wallets = customer_treasuries::Entity::find()
            .filter(
                customer_treasuries::Column::CustomerId.is_in(keys.iter().map(ToOwned::to_owned)),
            )
            .join(
                JoinType::InnerJoin,
                customer_treasuries::Relation::Wallets.def(),
            )
            .select_with(wallets::Entity)
            .all(conn)
            .await?;

        Ok(customer_wallets
            .into_iter()
            .map(|(customer_treasuries, wallets)| {
                (
                    customer_treasuries.customer_id,
                    wallets.into_iter().map(|wallet| wallet.address).collect(),
                )
            })
            .collect())
    }
}
