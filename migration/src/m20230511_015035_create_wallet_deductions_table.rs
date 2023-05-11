use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WalletDeductions::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(WalletDeductions::Id).uuid().primary_key())
                    .col(
                        ColumnDef::new(WalletDeductions::CustomerTreasury)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WalletDeductions::AssetId)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(WalletDeductions::Address).string())
                    .col(
                        ColumnDef::new(WalletDeductions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("default now()".to_string()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                IndexCreateStatement::new()
                    .name("wallet_deductions_customer_treasury_idx")
                    .table(WalletDeductions::Table)
                    .col(WalletDeductions::CustomerTreasury)
                    .index_type(IndexType::Hash)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                IndexCreateStatement::new()
                    .name("wallet_deductions_asset_id_idx")
                    .table(WalletDeductions::Table)
                    .col(WalletDeductions::AssetId)
                    .index_type(IndexType::BTree)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                IndexCreateStatement::new()
                    .name("wallet_deductions_address_idx")
                    .table(WalletDeductions::Table)
                    .col(WalletDeductions::Address)
                    .index_type(IndexType::Hash)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WalletDeductions::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum WalletDeductions {
    Table,
    Id,
    CustomerTreasury,
    AssetId,
    Address,
    CreatedAt,
}
