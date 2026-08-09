use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GeneralTransactions::Table)
                    .if_not_exists()
                    .col(pk_auto(GeneralTransactions::Id))
                    .col(integer(GeneralTransactions::WorkspaceId))
                    .col(integer(GeneralTransactions::BankAccountId))
                    .col(string(GeneralTransactions::TransactionType))
                    .col(string_null(GeneralTransactions::Categoria))
                    .col(double(GeneralTransactions::Valor))
                    .col(string(GeneralTransactions::TransactionDate))
                    .col(string_null(GeneralTransactions::Notes))
                    .col(string(GeneralTransactions::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_general_transactions_workspace")
                            .from(GeneralTransactions::Table, GeneralTransactions::WorkspaceId)
                            .to(Workspace::Table, Workspace::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_general_transactions_bank_account")
                            .from(GeneralTransactions::Table, GeneralTransactions::BankAccountId)
                            .to(BankAccounts::Table, BankAccounts::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GeneralTransactions::Table).to_owned())
            .await
    }
}

/// Fase 12 — núcleo de Finanças Gerais. Lançamento de fluxo de caixa geral
/// (`receita`/`despesa` nesta fatia — `parcela_divida`/`compra_ativo`/
/// `venda_ativo` do rascunho ficam para as fatias de `Liability`/link com
/// Portfolio). Entidade **paralela** à `transactions` do Portfolio (Fase
/// 10.2), decisão confirmada com o dono do projeto — evita acoplar
/// semânticas diferentes numa tabela que já alimenta
/// `get_portfolio_positions`. `BankAccountId` é obrigatório (todo lançamento
/// pertence a uma conta, conforme o rascunho). `Valor` é sempre positivo — o
/// sinal (soma ou subtrai do saldo) é implícito em `TransactionType`.
/// `Categoria` é texto livre por ora, sem a entidade `Category`/limite de
/// orçamento do rascunho ainda.
#[derive(DeriveIden)]
enum GeneralTransactions {
    Table,
    Id,
    WorkspaceId,
    BankAccountId,
    TransactionType,
    Categoria,
    Valor,
    TransactionDate,
    Notes,
    CreatedAt,
}

/// Só `Table`/`Id` — apontam FK pras tabelas donas, criadas em migrations
/// anteriores.
#[derive(DeriveIden)]
enum Workspace {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum BankAccounts {
    Table,
    Id,
}
