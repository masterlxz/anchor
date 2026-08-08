use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AssetValuations::Table)
                    .if_not_exists()
                    .col(pk_auto(AssetValuations::Id))
                    .col(integer(AssetValuations::AssetId))
                    .col(string(AssetValuations::ValuationDate))
                    .col(double(AssetValuations::Value))
                    .col(string(AssetValuations::Origin))
                    .col(string_null(AssetValuations::Notes))
                    .col(string(AssetValuations::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_asset_valuations_asset")
                            .from(AssetValuations::Table, AssetValuations::AssetId)
                            .to(Assets::Table, Assets::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AssetAttachments::Table)
                    .if_not_exists()
                    .col(pk_auto(AssetAttachments::Id))
                    .col(integer(AssetAttachments::AssetId))
                    .col(string(AssetAttachments::OriginalFileName))
                    .col(string(AssetAttachments::StoredRelativePath))
                    .col(big_integer(AssetAttachments::FileSizeBytes))
                    .col(string_null(AssetAttachments::ContentType))
                    .col(string_null(AssetAttachments::DocumentType))
                    .col(string(AssetAttachments::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_asset_attachments_asset")
                            .from(AssetAttachments::Table, AssetAttachments::AssetId)
                            .to(Assets::Table, Assets::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AssetAttachments::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(AssetValuations::Table).to_owned())
            .await
    }
}

/// Fase 10, item 8 — classe `imovel` (cadastro manual, sem fonte de dados
/// externa, ver PHASE.md item 8). Histórico de avaliações do imóvel ao
/// longo do tempo — `origin` já nasce como enum textual
/// (`manual`/`reajuste_automatico`) documentando a extensão futura prevista
/// no rascunho original, mas só `manual` é gravado por enquanto (mecanismo
/// de reajuste automático por % ainda não decidido). Cascade em `AssetId`
/// (diferente de `theses`/`transactions`, que não cascateiam pra `assets`):
/// aqui o histórico só faz sentido preso a esse imóvel específico, mesmo
/// raciocínio de `thesis_attachments` cascateando de `theses`.
#[derive(DeriveIden)]
enum AssetValuations {
    Table,
    Id,
    AssetId,
    ValuationDate,
    Value,
    Origin,
    Notes,
    CreatedAt,
}

/// Anexo de um ativo de cadastro manual (escritura, ITBI, IPTU pago, foto
/// etc.) — mesmo molde de `thesis_attachments` (arquivo vive fora do banco,
/// em `app_data_dir()/asset_attachments/{asset_id}/`, só metadados aqui).
/// `document_type` é texto livre (não enum) pra rotular o tipo de
/// documento sem travar o usuário numa lista fixa.
#[derive(DeriveIden)]
enum AssetAttachments {
    Table,
    Id,
    AssetId,
    OriginalFileName,
    StoredRelativePath,
    FileSizeBytes,
    ContentType,
    DocumentType,
    CreatedAt,
}

/// Só `Table`/`Id` — aponta FK pra tabela dona, criada em migration anterior.
#[derive(DeriveIden)]
enum Assets {
    Table,
    Id,
}
