use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(StockQuotes::Table)
                    .add_column(string_null(StockQuotes::Name))
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(StockQuotes::Table)
                    .add_column(string_null(StockQuotes::Exchange))
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(StockQuotes::Table)
                    .add_column(string_null(StockQuotes::Currency))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(StockQuotes::Table)
                    .drop_column(StockQuotes::Currency)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(StockQuotes::Table)
                    .drop_column(StockQuotes::Exchange)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(StockQuotes::Table)
                    .drop_column(StockQuotes::Name)
                    .to_owned(),
            )
            .await
    }
}

/// `name`/`exchange`/`currency` vêm do mesmo bloco `meta` do Yahoo que
/// `fetch_quotes` já lê pra `regularMarketPrice` — sem chamada extra.
/// Nullable porque linhas gravadas antes desta migration não têm valor, e
/// porque o Yahoo pode omitir qualquer um dos 3 mesmo numa resposta OK.
/// Alimenta o pré-preenchimento do cadastro de Ativo (Ação BR) em
/// `AssetSection.tsx`. Três `alter_table` separados porque SQLite/sea-query
/// não aceita mais de um `add_column` por `ALTER TABLE` (mesmo achado de
/// `add_token_usage_to_ai_message`).
#[derive(DeriveIden)]
enum StockQuotes {
    Table,
    Name,
    Exchange,
    Currency,
}
