use std::collections::HashMap;

use async_graphql::{dataloader::Loader as DataLoader, FieldError, Result};
use poem::async_trait;
use sea_orm::prelude::*;

use crate::{
    db::Connection,
    entities::{treasuries, wallets},
};

#[derive(Debug, Clone)]
pub struct WalletsLoader {
    pub db: Connection,
}

impl WalletsLoader {
    #[must_use]
    pub fn new(db: Connection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl DataLoader<Uuid> for WalletsLoader {
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
