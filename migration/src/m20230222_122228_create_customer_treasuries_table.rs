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
                    .table(CustomerTreasuries::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CustomerTreasuries::Id)
                            .uuid()
                            .primary_key()
                            .extra("default gen_random_uuid()".to_string()),
                    )
                    .col(
                        ColumnDef::new(CustomerTreasuries::CustomerId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CustomerTreasuries::TreasuryId)
                            .uuid()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(CustomerTreasuries::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("default now()".to_string()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("customer-treasuries-fk")
                            .from(CustomerTreasuries::Table, CustomerTreasuries::TreasuryId)
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
                    .name("customers_treasury_id_idx")
                    .table(CustomerTreasuries::Table)
                    .col(CustomerTreasuries::TreasuryId)
                    .index_type(IndexType::Hash)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                IndexCreateStatement::new()
                    .name("customers_customer_id_idx")
                    .table(CustomerTreasuries::Table)
                    .col(CustomerTreasuries::CustomerId)
                    .index_type(IndexType::Hash)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CustomerTreasuries::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum CustomerTreasuries {
    Table,
    Id,
    CustomerId,
    TreasuryId,
    CreatedAt,
}
