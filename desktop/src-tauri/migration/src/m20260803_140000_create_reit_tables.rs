use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ReitFundamentals::Table)
                    .if_not_exists()
                    .col(pk_auto(ReitFundamentals::Id))
                    .col(string(ReitFundamentals::Ticker))
                    .col(integer(ReitFundamentals::ReferenceYear))
                    .col(double(ReitFundamentals::Revenue))
                    .col(double_null(ReitFundamentals::RealEstatePropertyNet))
                    .col(double_null(ReitFundamentals::RealEstatePropertyAtCost))
                    .col(double(ReitFundamentals::StockholdersEquity))
                    .col(double_null(ReitFundamentals::NetIncome))
                    .col(double(ReitFundamentals::EpsDiluted))
                    .col(string(ReitFundamentals::Source))
                    .col(string(ReitFundamentals::FetchedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ReitManualIndicators::Table)
                    .if_not_exists()
                    .col(pk_auto(ReitManualIndicators::Id))
                    .col(string(ReitManualIndicators::Ticker))
                    .col(double_null(ReitManualIndicators::FfoPerShare))
                    .col(double_null(ReitManualIndicators::AffoPerShare))
                    .col(double_null(ReitManualIndicators::OccupancyPct))
                    .col(string(ReitManualIndicators::UpdatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_reit_manual_indicators_ticker")
                    .table(ReitManualIndicators::Table)
                    .col(ReitManualIndicators::Ticker)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ReitManualIndicators::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(ReitFundamentals::Table).to_owned())
            .await
    }
}

/// Fase 10, item 8 — REIT (equivalente americano do FII). Decisão explícita
/// (perguntada ao dono do projeto via `AskUserQuestion`): painel de
/// indicadores imobiliários, sem os 8 modelos de valuation/DCF que não
/// encaixam bem em imobiliário — por isso tabela própria, não
/// `stock_fundamentals`/`stock_dcf_fundamentals`.
///
/// `ReitFundamentals` — via SEC EDGAR (`sec_edgar.py::fetch_reit_fundamentals`),
/// mesmo padrão time-series de `stock_fundamentals` (um `INSERT` por fetch,
/// nunca sobrescrito, leitura pega a linha mais recente por `fetched_at`).
/// `real_estate_property_net`/`_at_cost` e `net_income` são nullable — nem
/// toda taxonomia de REIT reporta essas tags de forma consistente (achado
/// ao vivo: Simon Property não reporta `NetIncomeLoss`, usa `ProfitLoss`).
///
/// `ReitManualIndicators` — FFO/AFFO por ação e taxa de ocupação, os
/// indicadores "de verdade" de REIT que **não existem como tag XBRL**
/// (confirmado ao vivo contra 4 REITs reais antes de implementar — são
/// métricas non-GAAP só em texto/tabela do 10-K). Campo manual editável,
/// mesmo espírito do landbank do RNAV. Índice único em `ticker` — uma
/// linha por ativo, upsert.
#[derive(DeriveIden)]
enum ReitFundamentals {
    Table,
    Id,
    Ticker,
    ReferenceYear,
    Revenue,
    RealEstatePropertyNet,
    RealEstatePropertyAtCost,
    StockholdersEquity,
    NetIncome,
    EpsDiluted,
    Source,
    FetchedAt,
}

#[derive(DeriveIden)]
enum ReitManualIndicators {
    Table,
    Id,
    Ticker,
    FfoPerShare,
    AffoPerShare,
    OccupancyPct,
    UpdatedAt,
}
