use sea_orm_migration::{prelude::*, sea_orm::Statement};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        let solana_stmt = Statement::from_string(
            manager.get_database_backend(),
            r#"UPDATE wallets SET asset_id = 0 WHERE asset_id = 1;"#.to_string(),
        );

        db.execute(solana_stmt).await?;

        let polygon_stmt = Statement::from_string(
            manager.get_database_backend(),
            r#"UPDATE wallets SET asset_id = 3 WHERE asset_id = 2;"#.to_string(),
        );

        db.execute(polygon_stmt).await?;

        let eth_stmt = Statement::from_string(
            manager.get_database_backend(),
            r#"UPDATE wallets SET asset_id = 5 WHERE asset_id = 4;"#.to_string(),
        );

        db.execute(eth_stmt).await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
