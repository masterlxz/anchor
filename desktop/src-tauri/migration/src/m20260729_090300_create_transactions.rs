use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Transactions::Table)
                    .if_not_exists()
                    .col(pk_auto(Transactions::Id))
                    .col(integer(Transactions::PortfolioId))
                    .col(integer_null(Transactions::AssetId))
                    .col(integer_null(Transactions::CustodiaId))
                    .col(integer_null(Transactions::TransferToCustodiaId))
                    .col(string(Transactions::TransactionType))
                    .col(double_null(Transactions::Quantity))
                    .col(double_null(Transactions::UnitPrice))
                    .col(double(Transactions::TotalValue))
                    .col(double_null(Transactions::Fee))
                    .col(string(Transactions::TransactionDate))
                    .col(string_null(Transactions::Notes))
                    .col(string_null(Transactions::FiEmissor))
                    .col(string_null(Transactions::FiIndexador))
                    .col(double_null(Transactions::FiTaxaPercentual))
                    .col(string_null(Transactions::FiDataVencimento))
                    .col(string_null(Transactions::FiLiquidez))
                    .col(string(Transactions::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_transactions_portfolio")
                            .from(Transactions::Table, Transactions::PortfolioId)
                            .to(Portfolio::Table, Portfolio::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    // Sem `on_delete` nas 3 FKs abaixo (Asset/Custodia) de
                    // propósito: nem `assets` nem `custodia` ganham um
                    // comando de exclusão nesta fatia, então não há como
                    // isso disparar hoje — mas se um dia vierem a ganhar,
                    // apagar um cadastro de referência não deveria arrastar
                    // junto o histórico financeiro real (diferente da
                    // hierarquia de posse Workspace→Portfolio, que usa
                    // Cascade de propósito).
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_transactions_asset")
                            .from(Transactions::Table, Transactions::AssetId)
                            .to(Assets::Table, Assets::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_transactions_custodia")
                            .from(Transactions::Table, Transactions::CustodiaId)
                            .to(Custodia::Table, Custodia::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_transactions_transfer_to_custodia")
                            .from(Transactions::Table, Transactions::TransferToCustodiaId)
                            .to(Custodia::Table, Custodia::Id),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Transactions::Table).to_owned())
            .await
    }
}

/// Fase 10.2 — lançamentos de carteira (compra/venda/aporte/retirada/
/// provento/transferência entre custódias). `AssetId` é nulo só para
/// `aporte`/`retirada` (fluxo de caixa puro, sem ativo). `CustodiaId` é
/// nulo quando o usuário não quis detalhar onde o lançamento aconteceu
/// (Fase 10 item 9 é opcional nesta fatia, sem tela de gerenciamento
/// dedicada ainda). `TransferToCustodiaId` só é usado quando
/// `transaction_type = "transferencia"` (`CustodiaId` vira a origem).
///
/// Os campos `Fi*` (ver PHASE.md item 2 — Tesouro Direto/Renda Fixa
/// detalhados) ficam na transação, não no `Assets` — decisão da Sessão 33:
/// a taxa contratada/indexador/vencimento de um título de renda fixa é fixada
/// no momento da compra (rende flutua diariamente até lá), então duas
/// compras do "mesmo" título nominal (ex.: "Tesouro IPCA+ 2035" em datas
/// diferentes) podem legitimamente ter taxas contratadas diferentes — não é
/// uma propriedade fixa do ativo.
#[derive(DeriveIden)]
enum Transactions {
    Table,
    Id,
    PortfolioId,
    AssetId,
    CustodiaId,
    TransferToCustodiaId,
    TransactionType,
    Quantity,
    UnitPrice,
    TotalValue,
    Fee,
    TransactionDate,
    Notes,
    FiEmissor,
    FiIndexador,
    FiTaxaPercentual,
    FiDataVencimento,
    FiLiquidez,
    CreatedAt,
}

/// Só `Table`/`Id` — apontam FK pras tabelas donas, criadas em migrations
/// anteriores.
#[derive(DeriveIden)]
enum Portfolio {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Assets {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Custodia {
    Table,
    Id,
}
