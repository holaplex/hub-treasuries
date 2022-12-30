pub use sea_orm_migration::prelude::*;

mod m20221230_011552_create_treasuries_table;
mod m20221230_181041_create_project_treasuries_table;
mod m20221230_181519_wallets_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20221230_011552_create_treasuries_table::Migration),
            Box::new(m20221230_181041_create_project_treasuries_table::Migration),
            Box::new(m20221230_181519_wallets_table::Migration),
        ]
    }
}
