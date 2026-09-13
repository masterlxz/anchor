use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(StorageSettings::Table)
                    .if_not_exists()
                    .col(pk_auto(StorageSettings::Id))
                    .col(string(StorageSettings::Provider))
                    .col(string(StorageSettings::UpdatedAt))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(StorageSettings::Table).to_owned())
            .await
    }
}

/// Fase 15 — linha única (mesmo padrão "replace" de `finance_api_settings`,
/// `m20260829_141241`): `set_storage_settings` sempre `delete_many` + `insert`. `provider` é
/// `"local"` (único implementado) ou um dos 3 valores reservados
/// (`"self_hosted"`/`"managed_cloud"`/`"decentralized_vault"`, recusados por
/// `set_storage_settings` até ganharem implementação real).
#[derive(DeriveIden)]
enum StorageSettings {
    Table,
    Id,
    Provider,
    UpdatedAt,
}
