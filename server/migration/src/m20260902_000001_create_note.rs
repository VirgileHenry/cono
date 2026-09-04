use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // ── notes: current state, fully materialized ─────────────
        manager
            .create_table(
                Table::create()
                    .table(Notes::Table)
                    .col(ColumnDef::new(Notes::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Notes::Title).string().not_null())
                    .col(ColumnDef::new(Notes::Content).text().not_null().default(""))
                    .col(ColumnDef::new(Notes::Version).big_unsigned().not_null().default(0))
                    .col(ColumnDef::new(Notes::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(Notes::UpdatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        // ── note_edits: append-only op log ───────────────────────
        manager
            .create_table(
                Table::create()
                    .table(NoteEdits::Table)
                    .col(
                        ColumnDef::new(NoteEdits::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(NoteEdits::NoteId).uuid().not_null())
                    .col(ColumnDef::new(NoteEdits::Version).big_unsigned().not_null())
                    .col(ColumnDef::new(NoteEdits::Op).json_binary().not_null())
                    .col(ColumnDef::new(NoteEdits::CreatedAt).timestamp_with_time_zone().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_note_edits_note_id")
                            .from(NoteEdits::Table, NoteEdits::NoteId)
                            .to(Notes::Table, Notes::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // One version = one op per note, and fast history/compaction scans
        manager
            .create_index(
                Index::create()
                    .name("idx_note_edits_note_id_version")
                    .table(NoteEdits::Table)
                    .col(NoteEdits::NoteId)
                    .col(NoteEdits::Version)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(NoteEdits::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Notes::Table).to_owned()).await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Notes {
    Table,
    Id,
    Title,
    Content,
    Version,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum NoteEdits {
    Table,
    Id,
    NoteId,
    Version,
    Op,
    CreatedAt,
}
