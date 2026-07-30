use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(StockPriceHistory::Table)
                    .if_not_exists()
                    .col(pk_auto(StockPriceHistory::Id))
                    .col(string(StockPriceHistory::Ticker))
                    .col(string(StockPriceHistory::PriceDate))
                    .col(double(StockPriceHistory::ClosePrice))
                    .col(string(StockPriceHistory::Source))
                    .col(string(StockPriceHistory::FetchedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_stock_price_history_ticker_date")
                    .table(StockPriceHistory::Table)
                    .col(StockPriceHistory::Ticker)
                    .col(StockPriceHistory::PriceDate)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(StockPriceHistory::Table).to_owned())
            .await
    }
}

/// Série diária de fechamento (Fase 10.3, base pra reconstruir BMV/EMV mês a
/// mês do TWR/Dietz Modificado) — mesmo molde de `stock_dividend_payments`:
/// índice único `(ticker, price_date)` permite `INSERT OR IGNORE` no coletor
/// Python, então rodar o backfill de novo não duplica pregão já salvo, só
/// acrescenta dias novos. Coluna chamada `close_price` (não `close`, mesmo
/// padrão de `stock_quotes.price`) pra evitar ambiguidade com a palavra SQL.
#[derive(DeriveIden)]
enum StockPriceHistory {
    Table,
    Id,
    Ticker,
    PriceDate,
    ClosePrice,
    Source,
    FetchedAt,
}
