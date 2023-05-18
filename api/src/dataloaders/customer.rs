use std::collections::HashMap;

use async_graphql::{dataloader::Loader as DataLoader, FieldError, Result};
use poem::async_trait;
use sea_orm::{prelude::*, JoinType, QuerySelect};

use crate::{
    db::Connection,
    entities::{customer_treasuries, treasuries, wallets},
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
                customer_treasuries::Column::CustomerId
                    .is_in(keys.iter().map(ToOwned::to_owned))
                    .and(wallets::Column::Address.is_not_null()),
            )
            .join(
                JoinType::InnerJoin,
                customer_treasuries::Relation::Treasuries.def(),
            )
            .join(JoinType::InnerJoin, treasuries::Relation::Wallets.def())
            .select_with(wallets::Entity)
            .all(conn)
            .await?;

        Ok(customer_wallets
            .into_iter()
            .map(|(ct, wallets)| {
                let addresses = wallets
                    .into_iter()
                    .map(|wallet| {
                        wallet.address.ok_or_else(|| {
                            Self::Error::new(format!(
                                "Address is missing for wallet with ID {}",
                                wallet.id
                            ))
                        })
                    })
                    .collect::<Result<_>>()?;
                Ok((ct.customer_id, addresses))
            })
            .collect::<Result<_>>()?)
    }
}
