use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Achado ao vivo contra o banco real de dev: já existiam 2 linhas
        // pra (ticker="O", reference_year=2025) de testes manuais da REIT
        // original (Fase 10, item 8, antes da Fase 14 existir) — criar o
        // índice único direto quebraria a migration. Mantém só a linha mais
        // recente por `fetched_at` em cada `(ticker, reference_year)`.
        let db = manager.get_connection();
        db.execute_unprepared(
            "DELETE FROM reit_fundamentals WHERE id NOT IN ( \
                SELECT MAX(id) FROM reit_fundamentals GROUP BY ticker, reference_year \
            )",
        )
        .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_reit_fundamentals_unique")
                    .table(ReitFundamentals::Table)
                    .col(ReitFundamentals::Ticker)
                    .col(ReitFundamentals::ReferenceYear)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_reit_fundamentals_unique")
                    .table(ReitFundamentals::Table)
                    .to_owned(),
            )
            .await
    }
}

/// Fase 14.4 (Sessão 92) — achado ao vivo: `finance_api::reit::
/// collect_reit_fundamentals` porta o fetch pra Rust consumindo `GET
/// /v1/us-stocks/{ticker}/reit-fundamentals`, que devolve a série histórica
/// **completa** a cada chamada (Fase 1.11.2 do easybusiness, tabela
/// append-only do lado de lá) — diferente do coletor Python original, que só
/// buscava 1 ano por execução e fazia um `INSERT` cego sem se preocupar com
/// duplicata. Sem um índice único aqui, rodar o coletor 2x duplicaria **cada
/// ano já existente** a cada chamada (não só o ano novo), já que agora a
/// fonte sempre devolve tudo de novo. `ReitManualIndicators` já tinha esse
/// cuidado (`idx_reit_manual_indicators_ticker`); `ReitFundamentals` ficou
/// sem porque, na migration original, era escrito só via subprocess Python
/// (1 INSERT cego por execução, nunca 2x seguidas no mesmo teste).
#[derive(DeriveIden)]
enum ReitFundamentals {
    Table,
    Ticker,
    ReferenceYear,
}
