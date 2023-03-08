use std::collections::HashMap;

use async_graphql::{dataloader::Loader as DataLoader, FieldError, Result};
use poem::async_trait;
use sea_orm::{prelude::*, JoinType, QuerySelect};

use crate::{
    db::Connection,
    entities::{customer_treasuries, project_treasuries, treasuries},
};

#[derive(Debug, Clone)]
pub struct Loader {
    pub db: Connection,
}

impl Loader {
    #[must_use]
    pub fn new(db: Connection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl DataLoader<Uuid> for Loader {
    type Error = FieldError;
    type Value = treasuries::Model;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let treasuries = treasuries::Entity::find()
            .filter(treasuries::Column::Id.is_in(keys.iter().map(ToOwned::to_owned)))
            .all(self.db.get())
            .await?;

        Ok(treasuries
            .into_iter()
            .map(|treasury| (treasury.id, treasury))
            .collect())
    }
}

#[derive(Debug, Clone)]
pub struct CustomerLoader {
    pub db: Connection,
}

impl CustomerLoader {
    #[must_use]
    pub fn new(db: Connection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl DataLoader<Uuid> for CustomerLoader {
    type Error = FieldError;
    type Value = treasuries::Model;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let treasuries = customer_treasuries::Entity::find()
            .join(
                JoinType::InnerJoin,
                customer_treasuries::Relation::Treasury.def(),
            )
            .select_also(treasuries::Entity)
            .filter(
                customer_treasuries::Column::CustomerId.is_in(keys.iter().map(ToOwned::to_owned)),
            )
            .all(self.db.get())
            .await?;

        Ok(treasuries
            .into_iter()
            .filter_map(|(customer_treasury, treasury)| {
                treasury.map(|treasury| (customer_treasury.customer_id, treasury))
            })
            .collect())
    }
}

#[derive(Debug, Clone)]
pub struct ProjectLoader {
    pub db: Connection,
}

impl ProjectLoader {
    #[must_use]
    pub fn new(db: Connection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl DataLoader<Uuid> for ProjectLoader {
    type Error = FieldError;
    type Value = treasuries::Model;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let treasuries = project_treasuries::Entity::find()
            .join(
                JoinType::InnerJoin,
                project_treasuries::Relation::Treasury.def(),
            )
            .select_also(treasuries::Entity)
            .filter(project_treasuries::Column::ProjectId.is_in(keys.iter().map(ToOwned::to_owned)))
            .all(self.db.get())
            .await?;

        Ok(treasuries
            .into_iter()
            .filter_map(|(project_treasury, treasury)| {
                treasury.map(|treasury| (project_treasury.project_id, treasury))
            })
            .collect())
    }
}
