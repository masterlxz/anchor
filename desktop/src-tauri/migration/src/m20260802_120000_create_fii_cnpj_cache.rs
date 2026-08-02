use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(FiiCnpjCache::Table)
                    .if_not_exists()
                    .col(pk_auto(FiiCnpjCache::Id))
                    .col(string(FiiCnpjCache::Ticker))
                    .col(string(FiiCnpjCache::Cnpj))
                    .col(string(FiiCnpjCache::FundName))
                    .col(string(FiiCnpjCache::ResolvedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_fii_cnpj_cache_ticker")
                    .table(FiiCnpjCache::Table)
                    .col(FiiCnpjCache::Ticker)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(FiiCnpjCache::Table).to_owned())
            .await
    }
}

/// Fase 10, item 8, Sessão 43 — cache de `resolve_fii_cnpj` por ticker,
/// independente de `assets.cnpj`. Necessário pra tela de Pesquisa unificada
/// (pesquisar um FII não pressupõe tê-lo cadastrado como ativo da carteira,
/// diferente do fluxo de `AssetSection.tsx`) sem quebrar a promessa feita ao
/// dono do projeto na Sessão 42 — "depois de resolvido, a bolsai nunca mais
/// é chamada" — que só valia pro caminho de cadastro até aqui. Com este
/// cache, `main.py::main_fii_resolve_cnpj` consulta esta tabela primeiro;
/// só chama a bolsai numa pesquisa nova de um ticker nunca resolvido antes,
/// em qualquer uma das duas telas. Índice único em `ticker` — um ticker só
/// tem um CNPJ, `INSERT OR IGNORE` de novo é sempre um no-op esperado.
#[derive(DeriveIden)]
enum FiiCnpjCache {
    Table,
    Id,
    Ticker,
    Cnpj,
    FundName,
    ResolvedAt,
}
