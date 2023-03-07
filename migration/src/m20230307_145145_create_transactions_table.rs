use sea_orm_migration::{prelude::*, sea_query::extension::postgres::Type};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_type(
                Type::create()
                    .as_enum(TransactionStatus::Type)
                    .values([
                        TransactionStatus::Submitted,
                        TransactionStatus::Queued,
                        TransactionStatus::PendingAuthorization,
                        TransactionStatus::PendingSignature,
                        TransactionStatus::Broadcasting,
                        TransactionStatus::Pending3rdPartyManualApproval,
                        TransactionStatus::Pending3rdParty,
                        TransactionStatus::Confirming,
                        TransactionStatus::PartiallyCompleted,
                        TransactionStatus::PendingAmlScreening,
                        TransactionStatus::Completed,
                        TransactionStatus::Canceled,
                        TransactionStatus::Rejected,
                        TransactionStatus::Blocked,
                        TransactionStatus::Failed,
                        TransactionStatus::Pending,
                    ])
                    .to_owned(),
            )
            .await?;

        manager
            .create_type(
                Type::create()
                    .as_enum(EventType::Type)
                    .values([EventType::CreateDrop, EventType::MintDrop])
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Transactions::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Transactions::Id).string().primary_key())
                    .col(ColumnDef::new(Transactions::FullSignature).string())
                    .col(
                        ColumnDef::new(Transactions::Status)
                            .custom(TransactionStatus::Type)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Transactions::EventType)
                            .custom(EventType::Type)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Transactions::SignedMessageSignatures)
                            .array(ColumnType::Text)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Transactions::EventId).binary().not_null())
                    .col(
                        ColumnDef::new(Transactions::EventPayload)
                            .binary()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                IndexCreateStatement::new()
                    .name("tx_signatures_signature_idx")
                    .table(Transactions::Table)
                    .col(Transactions::FullSignature)
                    .index_type(IndexType::Hash)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Transactions::Table).to_owned())
            .await?;

        manager
            .drop_type(Type::drop().if_exists().name(EventType::Type).to_owned())
            .await?;

        manager
            .drop_type(
                Type::drop()
                    .if_exists()
                    .name(TransactionStatus::Type)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
pub enum Transactions {
    Table,
    Id,
    FullSignature,
    Status,
    SignedMessageSignatures,
    EventType,
    EventId,
    EventPayload,
}

pub enum EventType {
    Type,
    CreateDrop,
    MintDrop,
}

impl Iden for EventType {
    fn unquoted(&self, s: &mut dyn std::fmt::Write) {
        write!(s, "{}", match self {
            Self::Type => "event_type",
            Self::CreateDrop => "create_drop",
            Self::MintDrop => "mint_drop",
        })
        .unwrap();
    }
}

pub enum TransactionStatus {
    Type,
    Submitted,
    Queued,
    PendingAuthorization,
    PendingSignature,
    Broadcasting,
    Pending3rdPartyManualApproval,
    Pending3rdParty,
    Confirming,
    PartiallyCompleted,
    PendingAmlScreening,
    Completed,
    Canceled,
    Rejected,
    Blocked,
    Failed,
    Pending,
}

impl Iden for TransactionStatus {
    fn unquoted(&self, s: &mut dyn std::fmt::Write) {
        write!(s, "{}", match self {
            Self::Type => "transaction_status",
            Self::Submitted => "submitted",
            Self::Queued => "queued",
            Self::PendingAuthorization => "pending_authorization",
            Self::PendingSignature => "pending_signature",
            Self::Broadcasting => "broadcasting",
            Self::Pending3rdPartyManualApproval => "pending_3rd_party_manual_approval",
            Self::Pending3rdParty => "pending_3rd_party",
            Self::Confirming => "confirming",
            Self::PartiallyCompleted => "partially_completed",
            Self::PendingAmlScreening => "pending_aml_screening",
            Self::Completed => "completed",
            Self::Canceled => "canceled",
            Self::Rejected => "rejected",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
            Self::Pending => "pending",
        })
        .unwrap();
    }
}
