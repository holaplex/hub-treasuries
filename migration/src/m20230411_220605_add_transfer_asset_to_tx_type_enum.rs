use sea_orm_migration::{prelude::*, sea_query::extension::postgres::Type};

use crate::m20230403_190832_create_transactions_table::TxType;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_type(
                Type::alter()
                    .name(TxType::Type)
                    .add_value(Alias::new("transfer_mint"))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
