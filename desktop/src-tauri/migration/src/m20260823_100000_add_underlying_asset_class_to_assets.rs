use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Assets::Table)
                    .add_column(string_null(Assets::UnderlyingAssetClass))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Assets::Table)
                    .drop_column(Assets::UnderlyingAssetClass)
                    .to_owned(),
            )
            .await
    }
}

/// Fase 10, item 8, Sessão 86 — só relevante pra `bdr` (recibo B3 de algo
/// estrangeiro que não é necessariamente empresa operacional). Nullable,
/// mesmo padrão do `cnpj` (só FII): BDRs cadastrados antes desta migration
/// ficam com `null`, editável depois via `update_asset_underlying_class`.
/// Guarda um valor de `AssetClass` (restrito a um subconjunto no backend,
/// ver `commands::asset::BDR_UNDERLYING_CLASSES`) em vez de um enum próprio
/// — decisão do dono do projeto via `AskUserQuestion`, reaproveita os
/// rótulos que `ASSET_CLASS_LABELS` já tem no frontend.
#[derive(DeriveIden)]
enum Assets {
    Table,
    UnderlyingAssetClass,
}
