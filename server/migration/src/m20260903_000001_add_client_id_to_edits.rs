use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(NoteEdits::Table)
                    .add_column(
                        ColumnDef::new(NoteEdits::ClientId)
                            .uuid()
                            .not_null()
                            /* Backfill existing rows with the nil uuid: no live
                            connection can ever hold it, so old edits are simply
                            "foreign to everyone" — which is the correct semantic
                            for pre-migration history. */
                            .default(Expr::cust("'00000000-0000-0000-0000-000000000000'::uuid")),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(NoteEdits::Table)
                    .drop_column(NoteEdits::ClientId)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum NoteEdits {
    Table,
    ClientId,
}
