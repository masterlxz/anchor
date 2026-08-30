use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CvmDividendNoticeDocuments::Table)
                    .if_not_exists()
                    .col(pk_auto(CvmDividendNoticeDocuments::Id))
                    .col(integer(CvmDividendNoticeDocuments::CvmCode))
                    .col(string(CvmDividendNoticeDocuments::ProtocoloEntrega))
                    .col(boolean(CvmDividendNoticeDocuments::Matched))
                    .col(string_null(CvmDividendNoticeDocuments::Note))
                    .col(string(CvmDividendNoticeDocuments::ProcessedAt))
                    .to_owned(),
            )
            .await?;

        // Índice único: nunca reprocessar o mesmo documento (independente do
        // resultado) — mesmo espírito idempotente de `suggested_dividends`.
        manager
            .create_index(
                Index::create()
                    .name("idx_cvm_dividend_notice_documents_unique")
                    .table(CvmDividendNoticeDocuments::Table)
                    .col(CvmDividendNoticeDocuments::CvmCode)
                    .col(CvmDividendNoticeDocuments::ProtocoloEntrega)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CvmDividendNoticeDocuments::Table).to_owned())
            .await
    }
}

/// Fase 13.6 — ledger de idempotência pra `check_cvm_dividend_notices`
/// (`commands/cvm_dividend_notice.rs`): cada "Relatório Proventos" da CVM só
/// é baixado/extraído via IA uma vez, independente de ter virado sugestão
/// (`matched = true`) ou sido pulado por ambiguidade entre classes de ação
/// (`matched = false`, `note` explica o motivo). Não guarda o resultado da
/// extração em si — quem consome isso é `suggested_dividends`
/// (`source = "cvm"`) direto, essa tabela só evita retrabalho/gasto de IA
/// repetido no mesmo documento.
#[derive(DeriveIden)]
enum CvmDividendNoticeDocuments {
    Table,
    Id,
    CvmCode,
    ProtocoloEntrega,
    Matched,
    Note,
    ProcessedAt,
}
