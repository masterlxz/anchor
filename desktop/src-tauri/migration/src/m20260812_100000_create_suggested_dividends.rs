use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SuggestedDividends::Table)
                    .if_not_exists()
                    .col(pk_auto(SuggestedDividends::Id))
                    .col(integer(SuggestedDividends::PortfolioId))
                    .col(integer(SuggestedDividends::AssetId))
                    .col(string(SuggestedDividends::PaymentDate))
                    .col(double(SuggestedDividends::Amount))
                    .col(double(SuggestedDividends::Quantity))
                    .col(double(SuggestedDividends::Total))
                    .col(string(SuggestedDividends::Status))
                    .col(string_null(SuggestedDividends::ComDate))
                    .col(string(SuggestedDividends::Source))
                    .col(string(SuggestedDividends::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_suggested_dividends_portfolio")
                            .from(SuggestedDividends::Table, SuggestedDividends::PortfolioId)
                            .to(Portfolio::Table, Portfolio::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    // Sem `on_delete` no asset, mesmo motivo da tabela
                    // `transactions`: apagar um cadastro de referência não
                    // deve arrastar junto histórico financeiro real.
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_suggested_dividends_asset")
                            .from(SuggestedDividends::Table, SuggestedDividends::AssetId)
                            .to(Assets::Table, Assets::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // Índice único `(portfolio_id, asset_id, payment_date)`: a geração é
        // idempotente — rodar de novo não duplica sugestão que já existe,
        // mesmo espírito do índice único de `stock_dividend_payments`.
        manager
            .create_index(
                Index::create()
                    .name("idx_suggested_dividends_unique")
                    .table(SuggestedDividends::Table)
                    .col(SuggestedDividends::PortfolioId)
                    .col(SuggestedDividends::AssetId)
                    .col(SuggestedDividends::PaymentDate)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SuggestedDividends::Table).to_owned())
            .await
    }
}

/// Fase 13.6 — sugestões de lançamento de provento geradas a partir de
/// fonte externa (primeira versão: histórico do `stock_dividend_payments`,
/// que hoje só tem o "passado" via Yahoo Finance). O dono do projeto
/// confirma/edita/descarta cada uma numa tela de revisão; só a confirmação
/// escreve de verdade na `transactions` (como `provento`). `com_date`
/// (Data Com) fica nula até a confirmação — decisão da Sessão 76: nenhuma
/// fonte gratuita validada devolve Data Com com confiança, então é campo
/// manual.
#[derive(DeriveIden)]
enum SuggestedDividends {
    Table,
    Id,
    PortfolioId,
    AssetId,
    PaymentDate,
    Amount,
    Quantity,
    Total,
    Status,
    ComDate,
    Source,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Portfolio {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Assets {
    Table,
    Id,
}