use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MacroIndexMonthly::Table)
                    .if_not_exists()
                    .col(pk_auto(MacroIndexMonthly::Id))
                    .col(string(MacroIndexMonthly::IndexCode))
                    .col(string(MacroIndexMonthly::YearMonth))
                    .col(double(MacroIndexMonthly::ValuePct))
                    .col(string(MacroIndexMonthly::Source))
                    .col(string(MacroIndexMonthly::FetchedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_macro_index_monthly_unique")
                    .table(MacroIndexMonthly::Table)
                    .col(MacroIndexMonthly::IndexCode)
                    .col(MacroIndexMonthly::YearMonth)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MacroIndexMonthly::Table).to_owned())
            .await
    }
}

/// Fase 13.5 — série mensal dos benchmarks macro (`index_code`: "cdi" |
/// "ipca"), fonte Banco Central (SGS séries 4391/433, já em % mensal, sem
/// conta nenhuma). Índice único `(index_code, year_month)` permite `INSERT
/// OR REPLACE` no coletor Python (diferente do resto do coletor, que usa
/// IGNORE — a BCB pode revisar um valor recém-publicado do mês corrente).
/// Sem FK: é uma série global, não pertence a nenhum portfolio/asset.
/// IBOV/IVVB11 propositalmente NÃO usam esta tabela — reaproveitam
/// `stock_price_history` (são preço de ticker/índice via Yahoo, não %
/// pronto de uma fonte macro). IFIX/SMLL/IDIV ficam de fora desta fase por
/// falta de fonte histórica gratuita (ver PHASE.md item 13.5).
#[derive(DeriveIden)]
enum MacroIndexMonthly {
    Table,
    Id,
    IndexCode,
    YearMonth,
    ValuePct,
    Source,
    FetchedAt,
}
