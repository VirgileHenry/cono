pub use sea_orm_migration::prelude::*;

mod m20260902_000001_create_note;
mod m20260903_000001_add_client_id_to_edits;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260902_000001_create_note::Migration),
            Box::new(m20260903_000001_add_client_id_to_edits::Migration),
        ]
    }
}
