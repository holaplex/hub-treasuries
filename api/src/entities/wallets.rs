use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, poem_openapi::Object)]
#[sea_orm(table_name = "wallets")]
#[oai(rename = "Wallet")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub treasury_id: Uuid,
    pub asset_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub address: String,
    pub legacy_address: String,
    pub tag: String,
    pub created_at: DateTime,
    pub removed_at: Option<DateTime>,
    pub created_by: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::treasuries::Entity",
        from = "Column::TreasuryId",
        to = "super::treasuries::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Treasuries,
}

impl Related<super::treasuries::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Treasuries.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
