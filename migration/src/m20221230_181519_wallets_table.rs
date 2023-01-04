use sea_orm_migration::prelude::*;

use crate::m20221230_011552_create_treasuries_table::Treasuries;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Wallets::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Wallets::TreasuryId).uuid().not_null())
                    .col(ColumnDef::new(Wallets::Address).string().not_null())
                    .col(ColumnDef::new(Wallets::LegacyAddress).string().not_null())
                    .col(ColumnDef::new(Wallets::Tag).string().not_null())
                    .col(
                        ColumnDef::new(Wallets::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("default now()".to_string()),
                    )
                    .col(ColumnDef::new(Wallets::RemovedAt).timestamp())
                    .col(ColumnDef::new(Wallets::CreatedBy).uuid().not_null())
                    .primary_key(
                        Index::create()
                            .col(Wallets::TreasuryId)
                            .col(Wallets::Address),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("wallets_treasury_id_fk")
                            .from(Wallets::Table, Wallets::TreasuryId)
                            .to(Treasuries::Table, Treasuries::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                IndexCreateStatement::new()
                    .name("wallets_treasury_id_idx")
                    .table(Wallets::Table)
                    .col(Wallets::TreasuryId)
                    .index_type(IndexType::Hash)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Wallets::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Wallets {
    Table,
    TreasuryId,
    Address,
    LegacyAddress,
    Tag,
    CreatedBy,
    CreatedAt,
    RemovedAt,
}
