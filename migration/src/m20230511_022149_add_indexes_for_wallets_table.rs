use sea_orm_migration::prelude::*;

use crate::m20221230_181519_wallets_table::Wallets;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                IndexCreateStatement::new()
                    .name("wallets_asset_id_idx")
                    .table(Wallets::Table)
                    .col(Wallets::AssetId)
                    .index_type(IndexType::BTree)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                IndexCreateStatement::new()
                    .name("wallets_address_idx")
                    .table(Wallets::Table)
                    .col(Wallets::Address)
                    .index_type(IndexType::Hash)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
