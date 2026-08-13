use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Conciliação (Sessão 77.2): um provento esperado manual (`source =
        // "manual"`, data futura) e a sugestão automática (`source` da
        // fonte, mesma data) precisam poder coexistir quando os valores
        // divergirem — o dono do projeto compara os dois na tela. O índice
        // antigo `(portfolio, asset, payment_date)` impediria isso, então vira
        // `(portfolio, asset, payment_date, source)`. A geração continua
        // idempotente: ela mesmo nunca cria duplicada da mesma `source`.
        manager
            .drop_index(
                Index::drop()
                    .name("idx_suggested_dividends_unique")
                    .table(SuggestedDividends::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_suggested_dividends_unique")
                    .table(SuggestedDividends::Table)
                    .col(SuggestedDividends::PortfolioId)
                    .col(SuggestedDividends::AssetId)
                    .col(SuggestedDividends::PaymentDate)
                    .col(SuggestedDividends::Source)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_suggested_dividends_unique")
                    .table(SuggestedDividends::Table)
                    .to_owned(),
            )
            .await?;

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
}

#[derive(DeriveIden)]
enum SuggestedDividends {
    Table,
    PortfolioId,
    AssetId,
    PaymentDate,
    Source,
}