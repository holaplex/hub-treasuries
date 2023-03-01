use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Wallets::Table)
                    .drop_column(Wallets::AssetId)
                    .add_column_if_not_exists(ColumnDef::new(Wallets::AssetId).integer().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Wallets::Table)
                    .add_column_if_not_exists(ColumnDef::new(Wallets::AssetId).string().not_null())
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum Wallets {
    Table,
    AssetId,
}
