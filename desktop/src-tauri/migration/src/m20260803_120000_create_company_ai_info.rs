use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CompanyAiInfo::Table)
                    .if_not_exists()
                    .col(pk_auto(CompanyAiInfo::Id))
                    .col(string(CompanyAiInfo::Ticker))
                    .col(string(CompanyAiInfo::AssetClass))
                    .col(text(CompanyAiInfo::Content))
                    .col(string(CompanyAiInfo::Provider))
                    .col(string(CompanyAiInfo::Model))
                    .col(string(CompanyAiInfo::GeneratedAt))
                    .col(string_null(CompanyAiInfo::RegenerateReason))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CompanyAiInfo::Table).to_owned())
            .await
    }
}

/// Resumo "Sobre o ativo" gerado por IA (Fase 10, item 8, Sessão 60) — uma
/// row por `(ticker, asset_class)` (upsert feito na camada de comando, sem
/// constraint de unicidade no banco, mesmo padrão de `stock_notes`).
/// `asset_class` entra na chave porque o enquadramento do prompt muda por
/// classe (empresa vs. fundo vs. protocolo vs. commodity) e evita
/// ambiguidade se um ticker colidir entre classes diferentes.
#[derive(DeriveIden)]
enum CompanyAiInfo {
    Table,
    Id,
    Ticker,
    AssetClass,
    Content,
    Provider,
    Model,
    GeneratedAt,
    RegenerateReason,
}
