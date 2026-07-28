use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(StockDcfFundamentals::Table)
                    .add_column(double_null(StockDcfFundamentals::Inventory))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(StockDcfFundamentals::Table)
                    .drop_column(StockDcfFundamentals::Inventory)
                    .to_owned(),
            )
            .await
    }
}

/// `inventory` (Estoques, conta "1.01.04" do BPA) — nullable como
/// `tax_rate`/`revenue`: mesmo raciocínio de robustez dos outros campos
/// desta tabela, mesmo já sendo hoje uma dependência implícita obrigatória
/// de `nwc_change`. Pra auto-fill do RNAV (Fase 2.5).
#[derive(DeriveIden)]
enum StockDcfFundamentals {
    Table,
    Inventory,
}
