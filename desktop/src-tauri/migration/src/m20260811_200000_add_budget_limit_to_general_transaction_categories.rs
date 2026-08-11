use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(GeneralTransactionCategories::Table)
                    .add_column(double_null(GeneralTransactionCategories::LimiteMensal))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(GeneralTransactionCategories::Table)
                    .drop_column(GeneralTransactionCategories::LimiteMensal)
                    .to_owned(),
            )
            .await
    }
}

/// Fase 12 — limite de orçamento mensal por categoria (Sessão 75), fatia
/// deixada de fora de propósito na criação da tabela
/// (`m20260809_160000_create_general_transaction_categories`). Valor fixo
/// recorrente (não um valor por mês/ano específico) — decisão confirmada
/// com o dono do projeto.
#[derive(DeriveIden)]
enum GeneralTransactionCategories {
    Table,
    LimiteMensal,
}
