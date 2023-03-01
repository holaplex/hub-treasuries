pub use sea_orm_migration::prelude::*;

mod m20221230_011552_create_treasuries_table;
mod m20221230_181041_create_project_treasuries_table;
mod m20221230_181519_wallets_table;
mod m20230222_122228_create_customer_treasuries_table;
mod m20230301_124216_add_project_id_to_customer_treasuries;
mod m20230301_161016_change_asset_id_to_integer_on_wallets;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20221230_011552_create_treasuries_table::Migration),
            Box::new(m20221230_181041_create_project_treasuries_table::Migration),
            Box::new(m20221230_181519_wallets_table::Migration),
            Box::new(m20230222_122228_create_customer_treasuries_table::Migration),
            Box::new(m20230301_161016_change_asset_id_to_integer_on_wallets::Migration),
            Box::new(m20230301_124216_add_project_id_to_customer_treasuries::Migration),
        ]
    }
}
