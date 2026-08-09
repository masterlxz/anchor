use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Liabilities::Table)
                    .if_not_exists()
                    .col(pk_auto(Liabilities::Id))
                    .col(integer(Liabilities::WorkspaceId))
                    .col(integer(Liabilities::BankAccountId))
                    .col(integer_null(Liabilities::LinkedAssetId))
                    .col(string(Liabilities::Nome))
                    .col(string(Liabilities::Titular))
                    .col(double(Liabilities::Principal))
                    .col(double(Liabilities::TaxaPercentual))
                    .col(string(Liabilities::Indexador))
                    .col(integer(Liabilities::PrazoMeses))
                    .col(string(Liabilities::SistemaAmortizacao))
                    .col(string(Liabilities::DataInicio))
                    .col(string(Liabilities::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_liabilities_workspace")
                            .from(Liabilities::Table, Liabilities::WorkspaceId)
                            .to(Workspace::Table, Workspace::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_liabilities_bank_account")
                            .from(Liabilities::Table, Liabilities::BankAccountId)
                            .to(BankAccounts::Table, BankAccounts::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    // Sem `on_delete` — mesmo padrão de `fk_transactions_asset`
                    // (`m20260729_090300_create_transactions.rs`): `assets`
                    // não tem comando de exclusão hoje, e mesmo se ganhasse
                    // um, apagar o cadastro de referência não deveria
                    // arrastar uma dívida real.
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_liabilities_linked_asset")
                            .from(Liabilities::Table, Liabilities::LinkedAssetId)
                            .to(Assets::Table, Assets::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(LiabilityInstallments::Table)
                    .if_not_exists()
                    .col(pk_auto(LiabilityInstallments::Id))
                    .col(integer(LiabilityInstallments::LiabilityId))
                    .col(integer(LiabilityInstallments::GeneralTransactionId))
                    .col(integer(LiabilityInstallments::NumeroParcela))
                    .col(double(LiabilityInstallments::ValorParcela))
                    .col(double(LiabilityInstallments::ValorJuros))
                    .col(double(LiabilityInstallments::ValorAmortizacao))
                    .col(double(LiabilityInstallments::SaldoDevedorApos))
                    .col(string(LiabilityInstallments::DataVencimento))
                    .col(string(LiabilityInstallments::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_liability_installments_liability")
                            .from(LiabilityInstallments::Table, LiabilityInstallments::LiabilityId)
                            .to(Liabilities::Table, Liabilities::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_liability_installments_general_transaction")
                            .from(
                                LiabilityInstallments::Table,
                                LiabilityInstallments::GeneralTransactionId,
                            )
                            .to(GeneralTransactions::Table, GeneralTransactions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(LiabilityInstallments::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Liabilities::Table).to_owned())
            .await
    }
}

/// Fase 12 — dívida/passivo com geração automática de parcelas (Price ou
/// SAC). `TaxaPercentual` é sempre **mensal** (precisa ser inequívoco pra
/// matemática de amortização — ver `domain::liability`). `LinkedAssetId`
/// aponta pro ativo do Portfolio que a dívida financiou (opcional, decisão
/// confirmada com o dono do projeto): guardado desde já, mas sem cálculo de
/// retorno alavancado ainda — fica pra uma fatia futura.
#[derive(DeriveIden)]
enum Liabilities {
    Table,
    Id,
    WorkspaceId,
    BankAccountId,
    LinkedAssetId,
    Nome,
    Titular,
    Principal,
    TaxaPercentual,
    Indexador,
    PrazoMeses,
    SistemaAmortizacao,
    DataInicio,
    CreatedAt,
}

/// Uma linha por parcela gerada na criação da dívida. `GeneralTransactionId`
/// é o registro de fluxo de caixa de fato (debita a conta bancária) — esta
/// tabela só guarda o split juros/amortização e o saldo devedor após aquela
/// parcela, usado por `compute_outstanding_balance` pro resumo de dívida
/// total.
#[derive(DeriveIden)]
enum LiabilityInstallments {
    Table,
    Id,
    LiabilityId,
    GeneralTransactionId,
    NumeroParcela,
    ValorParcela,
    ValorJuros,
    ValorAmortizacao,
    SaldoDevedorApos,
    DataVencimento,
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

#[derive(DeriveIden)]
enum Assets {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum GeneralTransactions {
    Table,
    Id,
}
