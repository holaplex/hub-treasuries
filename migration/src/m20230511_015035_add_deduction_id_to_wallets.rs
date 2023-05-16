use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("ALTER TABLE WALLETS DROP CONSTRAINT wallets_pkey")
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Wallets::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(Wallets::Id)
                            .uuid()
                            .primary_key()
                            .extra("default gen_random_uuid()".to_string()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Wallets::Table)
                    .add_column_if_not_exists(ColumnDef::new(Wallets::DeductionId).uuid().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Wallets::Table)
                    .modify_column(ColumnDef::new(Alias::new("address")).string().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Wallets::Table)
                    .drop_column(Alias::new("legacy_address"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Wallets::Table)
                    .drop_column(Alias::new("tag"))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("ALTER TABLE WALLETS DROP CONSTRAINT wallets_pkey")
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Wallets::Table)
                    .add_column_if_not_exists(ColumnDef::new(Wallets::Tag).string().not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Wallets::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(Wallets::LegacyAddress).string().not_null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Wallets::Table)
                    .drop_column(Alias::new("deduction_id"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Wallets::Table)
                    .drop_column(Alias::new("id"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Wallets::Table)
                    .modify_column(
                        ColumnDef::new(Alias::new("address"))
                            .string()
                            .primary_key()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum Wallets {
    Table,
    Id,
    DeductionId,
    Tag,
    LegacyAddress,
}
