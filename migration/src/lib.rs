pub use sea_orm_migration::prelude::*;

mod m20221230_011552_create_treasuries_table;
mod m20221230_181041_create_project_treasuries_table;
mod m20221230_181519_wallets_table;
mod m20230222_122228_create_customer_treasuries_table;
mod m20230301_124216_add_project_id_to_customer_treasuries;
mod m20230301_161016_change_asset_id_to_integer_on_wallets;
mod m20230331_133153_remove_treasury_id_as_pk_from_wallets;
mod m20230403_190832_create_transactions_table;
mod m20230411_220605_add_transfer_asset_to_tx_type_enum;
mod m20230510_162853_change_datatype_to_tz_utc;
mod m20230511_015035_add_deduction_id_to_wallets;
mod m20230511_022149_add_indexes_for_wallets_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20221230_011552_create_treasuries_table::Migration),
            Box::new(m20221230_181041_create_project_treasuries_table::Migration),
            Box::new(m20221230_181519_wallets_table::Migration),
            Box::new(m20230222_122228_create_customer_treasuries_table::Migration),
            Box::new(m20230301_124216_add_project_id_to_customer_treasuries::Migration),
            Box::new(m20230301_161016_change_asset_id_to_integer_on_wallets::Migration),
            Box::new(m20230331_133153_remove_treasury_id_as_pk_from_wallets::Migration),
            Box::new(m20230403_190832_create_transactions_table::Migration),
            Box::new(m20230411_220605_add_transfer_asset_to_tx_type_enum::Migration),
            Box::new(m20230510_162853_change_datatype_to_tz_utc::Migration),
            Box::new(m20230511_015035_add_deduction_id_to_wallets::Migration),
            Box::new(m20230511_022149_add_indexes_for_wallets_table::Migration),
        ]
    }
}
