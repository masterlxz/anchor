use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Theses::Table)
                    .if_not_exists()
                    .col(pk_auto(Theses::Id))
                    .col(integer(Theses::WorkspaceId))
                    .col(integer_null(Theses::AssetId))
                    .col(string(Theses::Title))
                    .col(text(Theses::ContentMarkdown))
                    .col(string(Theses::CreatedAt))
                    .col(string(Theses::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_theses_workspace")
                            .from(Theses::Table, Theses::WorkspaceId)
                            .to(Workspace::Table, Workspace::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    // Sem `on_delete` pra Asset, mesmo raciocínio de
                    // `watchlist_items`/`transactions`: apagar um cadastro de
                    // referência não deveria arrastar a tese.
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_theses_asset")
                            .from(Theses::Table, Theses::AssetId)
                            .to(Assets::Table, Assets::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ThesisAttachments::Table)
                    .if_not_exists()
                    .col(pk_auto(ThesisAttachments::Id))
                    .col(integer(ThesisAttachments::ThesisId))
                    .col(string(ThesisAttachments::OriginalFileName))
                    .col(string(ThesisAttachments::StoredRelativePath))
                    .col(big_integer(ThesisAttachments::FileSizeBytes))
                    .col(string_null(ThesisAttachments::ContentType))
                    .col(string(ThesisAttachments::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_thesis_attachments_thesis")
                            .from(ThesisAttachments::Table, ThesisAttachments::ThesisId)
                            .to(Theses::Table, Theses::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ThesisAttachments::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Theses::Table).to_owned())
            .await
    }
}

/// Fase 10.5 — tese de investimento, vinculável a um ativo específico
/// (`asset_id` preenchido) ou global/macro (`asset_id` nulo). Pertence ao
/// Workspace, mesma hierarquia de posse de `Portfolio`/`Watchlists` (Cascade).
#[derive(DeriveIden)]
enum Theses {
    Table,
    Id,
    WorkspaceId,
    AssetId,
    Title,
    ContentMarkdown,
    CreatedAt,
    UpdatedAt,
}

/// Anexo de uma tese (PDF/planilha/imagem) — morre com a tese (Cascade).
/// Arquivo em si vive fora do banco, copiado pro diretório de dados do app;
/// `stored_relative_path` guarda só o caminho relativo a `app_data_dir()`.
#[derive(DeriveIden)]
enum ThesisAttachments {
    Table,
    Id,
    ThesisId,
    OriginalFileName,
    StoredRelativePath,
    FileSizeBytes,
    ContentType,
    CreatedAt,
}

/// Só `Table`/`Id` — apontam FK pras tabelas donas, criadas em migrations
/// anteriores.
#[derive(DeriveIden)]
enum Workspace {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Assets {
    Table,
    Id,
}
