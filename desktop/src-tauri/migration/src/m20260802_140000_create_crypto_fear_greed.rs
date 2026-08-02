use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CryptoFearGreed::Table)
                    .if_not_exists()
                    .col(pk_auto(CryptoFearGreed::Id))
                    .col(integer(CryptoFearGreed::Value))
                    .col(string(CryptoFearGreed::Classification))
                    .col(string(CryptoFearGreed::ReadingDate))
                    .col(string(CryptoFearGreed::Source))
                    .col(string(CryptoFearGreed::FetchedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_crypto_fear_greed_reading_date")
                    .table(CryptoFearGreed::Table)
                    .col(CryptoFearGreed::ReadingDate)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CryptoFearGreed::Table).to_owned())
            .await
    }
}

/// Fase 10, item 8, Sessão 51 — Fear & Greed Index (alternative.me,
/// `GET /fng/`, público sem chave, confirmado ao vivo 2026-08-02), pedido
/// explícito do dono do projeto pra aparecer em "toda tela que for cripto".
/// Diferente de `crypto_indicators` (Fase 3, por `coin`, calibrado só pra
/// ETH — ver domain/crypto_score.rs) — este índice é global de mercado, não
/// por moeda, então não tem coluna `coin`. Uma linha por dia (`reading_date`
/// único, mesmo padrão `INSERT OR IGNORE` de `stock_price_history`) —
/// buscado de novo a cada refresh de qualquer ticker cripto
/// (`collect_crypto_ticker`), mas só grava a primeira vez no dia.
#[derive(DeriveIden)]
enum CryptoFearGreed {
    Table,
    Id,
    Value,
    Classification,
    ReadingDate,
    Source,
    FetchedAt,
}
