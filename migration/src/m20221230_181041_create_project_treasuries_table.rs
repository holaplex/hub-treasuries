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
                    .table(ProjectTreasuries::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ProjectTreasuries::ProjectId)
                            .uuid()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ProjectTreasuries::TreasuryId)
                            .uuid()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(ProjectTreasuries::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("default now()".to_string()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("proj-treasuries-fk")
                            .from(ProjectTreasuries::Table, ProjectTreasuries::TreasuryId)
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
                    .name("proj_treasuries_id_idx")
                    .table(ProjectTreasuries::Table)
                    .col(ProjectTreasuries::TreasuryId)
                    .index_type(IndexType::Hash)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ProjectTreasuries::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum ProjectTreasuries {
    Table,
    ProjectId,
    TreasuryId,
    CreatedAt,
}
