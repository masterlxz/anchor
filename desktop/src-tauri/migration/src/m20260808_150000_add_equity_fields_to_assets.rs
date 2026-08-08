use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Três `alter_table` separados — SQLite/sea-query rejeita mais de um
        // `add_column` numa `ALTER TABLE` só (mesmo achado das migrations
        // `add_token_usage_to_ai_message`/`add_details_to_stock_quotes`).
        manager
            .alter_table(
                Table::alter()
                    .table(Assets::Table)
                    .add_column(double_null(Assets::EquitySharesOwned))
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Assets::Table)
                    .add_column(double_null(Assets::EquityTotalShares))
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Assets::Table)
                    .add_column(double_null(Assets::EquityCompanyValuation))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Assets::Table)
                    .drop_column(Assets::EquitySharesOwned)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Assets::Table)
                    .drop_column(Assets::EquityTotalShares)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Assets::Table)
                    .drop_column(Assets::EquityCompanyValuation)
                    .to_owned(),
            )
            .await
    }
}

/// Fase 10, item 8 — classe `empresa_nao_listada` (equity privado, mesmo
/// esqueleto `AtivoManual` do imóvel: histórico de avaliação + anexos, ver
/// `m20260808_100000_create_asset_valuations_and_attachments`). Estes 3
/// campos nullable em `assets` (mesmo padrão do `cnpj`, só usado por FII)
/// guardam a participação societária em si — quantidade de ações do
/// titular, total de ações emitidas pela empresa e o valuation total mais
/// recente, editáveis depois via `update_asset_equity` (rodadas de
/// investimento mudam esses números com o tempo). `percentual` **não** é
/// coluna própria — decisão explícita da sessão (rascunho da Sessão 30
/// deixava em aberto se seria validado ou digitado livre): calculado sempre
/// como `equity_shares_owned / equity_total_shares` no frontend, nunca
/// gravado, pra nunca dessincronizar dos 2 campos-fonte.
#[derive(DeriveIden)]
enum Assets {
    Table,
    EquitySharesOwned,
    EquityTotalShares,
    EquityCompanyValuation,
}
