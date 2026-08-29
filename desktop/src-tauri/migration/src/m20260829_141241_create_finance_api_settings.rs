use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(FinanceApiSettings::Table)
                    .if_not_exists()
                    .col(pk_auto(FinanceApiSettings::Id))
                    .col(string(FinanceApiSettings::Mode))
                    .col(string_null(FinanceApiSettings::RemoteUrl))
                    .col(string(FinanceApiSettings::UpdatedAt))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(FinanceApiSettings::Table).to_owned())
            .await
    }
}

/// Fase 14.3 — linha única (padrão "replace" de `commands::home_layout`, sem upsert por id
/// fixo): `set_finance_api_settings` sempre `delete_many` + `insert`. `mode` é `"local"` ou
/// `"remote"`; `remote_url` só é relevante quando `mode == "remote"`. A chave remota nunca vai
/// pra cá — fica no keyring, username fixo `"finance_api_remote"` (não reaproveita o esquema
/// `"{provider}:{id}"` de `ai_api_key` — schema errado pro caso, achado da Sessão 89).
#[derive(DeriveIden)]
enum FinanceApiSettings {
    Table,
    Id,
    Mode,
    RemoteUrl,
    UpdatedAt,
}
