use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Transactions::Table)
                    .add_column(string_null(Transactions::PaymentType))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Transactions::Table)
                    .drop_column(Transactions::PaymentType)
                    .to_owned(),
            )
            .await
    }
}

/// Fase 13.4 — só significativo quando `transaction_type = "provento"`.
/// Nullable (ao contrário de `suggested_dividends.payment_type`): lançamentos
/// `provento` antigos (criados antes desta migration, ou criados direto pelo
/// formulário "New transaction" sem passar pela tela de sugestões) não têm
/// esse dado — tratados como `"dividendo"` (sem retenção) na agregação
/// (`domain::proventos::net_total`, `None` cai no mesmo ramo de
/// `"dividendo"`).
#[derive(DeriveIden)]
enum Transactions {
    Table,
    PaymentType,
}
