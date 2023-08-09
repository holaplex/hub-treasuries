use sea_orm_migration::{prelude::*, sea_query::extension::postgres::Type};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_type(
                Type::create()
                    .as_enum(TxType::Type)
                    .values([
                        TxType::CreateDrop,
                        TxType::MintEdition,
                        TxType::UpdateMetadata,
                    ])
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Transactions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Transactions::FireblocksId)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Transactions::Signature).string().not_null())
                    .col(
                        ColumnDef::new(Transactions::TxType)
                            .custom(TxType::Type)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Transactions::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("default now()".to_string()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Transactions::Table).to_owned())
            .await?;

        manager
            .drop_type(Type::drop().if_exists().name(TxType::Type).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Transactions {
    Table,
    FireblocksId,
    Signature,
    TxType,
    CreatedAt,
}

pub enum TxType {
    Type,
    CreateDrop,
    MintEdition,
    UpdateMetadata,
}

impl Iden for TxType {
    fn unquoted(&self, s: &mut dyn std::fmt::Write) {
        s.write_str(match self {
            Self::Type => "tx_type",
            Self::CreateDrop => "create_drop",
            Self::MintEdition => "mint_edition",
            Self::UpdateMetadata => "update_metadata",
        })
        .unwrap();
    }
}
