use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(SuggestedDividends::Table)
                    .add_column(string(SuggestedDividends::PaymentType).default("dividendo"))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(SuggestedDividends::Table)
                    .drop_column(SuggestedDividends::PaymentType)
                    .to_owned(),
            )
            .await
    }
}

/// Fase 13.4 — Dividendo vs. JSCP (juros sobre capital próprio), com 15% de
/// IR retido na fonte só no JSCP (`domain::proventos::net_total`). Yahoo não
/// distingue os dois tipos, então toda sugestão automática nasce
/// `"dividendo"` por padrão; o dono do projeto pode corrigir pra `"jscp"` no
/// diálogo de Confirmar ou no de Registrar provento futuro. NOT NULL com
/// default (ao contrário de `transactions.payment_type`, nullable) porque
/// aqui o valor sempre é conhecido no momento do insert (gerado ou manual).
#[derive(DeriveIden)]
enum SuggestedDividends {
    Table,
    PaymentType,
}
