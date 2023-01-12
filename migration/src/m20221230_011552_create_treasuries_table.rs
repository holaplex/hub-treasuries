use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Treasuries::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Treasuries::Id)
                            .uuid()
                            .primary_key()
                            .extra("default gen_random_uuid()".to_string()),
                    )
                    .col(
                        ColumnDef::new(Treasuries::VaultId)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Treasuries::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("default now()".to_string()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                IndexCreateStatement::new()
                    .name("treasuries_vault_id_idx")
                    .table(Treasuries::Table)
                    .col(Treasuries::VaultId)
                    .index_type(IndexType::Hash)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Treasuries::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Treasuries {
    Table,
    Id,
    VaultId,
    CreatedAt,
}
