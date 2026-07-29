use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Assets::Table)
                    .if_not_exists()
                    .col(pk_auto(Assets::Id))
                    .col(string(Assets::Ticker))
                    .col(string(Assets::Name))
                    .col(string(Assets::AssetClass))
                    .col(string(Assets::Currency))
                    .col(string_null(Assets::Exchange))
                    .col(string(Assets::ExposureType))
                    .col(string(Assets::ExposureValue))
                    .col(string(Assets::CreatedAt))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Assets::Table).to_owned())
            .await
    }
}

/// Fase 10.2 (primeira fatia, escopo Sessão 29) — catálogo de ativos
/// negociáveis/registráveis num Portfolio. `AssetClass` por ora só cobre
/// `acao_br` | `acao_internacional` | `tesouro_direto` | `renda_fixa`; as
/// classes expandidas na Sessão 30 (FII/REIT/ETF/cripto/metal/imóvel/
/// empresa não listada, ver PHASE.md item 8) ficam para uma fatia futura —
/// nenhuma decisão em aberto delas (fonte de cotação de metal, reajuste de
/// imóvel, etc.) precisa ser resolvida ainda.
///
/// Modelo de exposição simplificado (decidido com o dono do projeto,
/// Sessão 33): um valor só por ativo, não o split percentual multi-país do
/// rascunho original — `ExposureType` é `pais` (`ExposureValue` = código do
/// país, ex. "BR"/"US") ou `categoria_especial` (`ExposureValue` = "cripto",
/// "metal_ouro" etc.). Evolui para peso % entre vários países só quando/se
/// isso vier a ser realmente necessário (ver PHASE.md item 8).
#[derive(DeriveIden)]
enum Assets {
    Table,
    Id,
    Ticker,
    Name,
    AssetClass,
    Currency,
    Exchange,
    ExposureType,
    ExposureValue,
    CreatedAt,
}
