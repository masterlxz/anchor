use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BankAccounts::Table)
                    .if_not_exists()
                    .col(pk_auto(BankAccounts::Id))
                    .col(integer(BankAccounts::WorkspaceId))
                    .col(string(BankAccounts::Nome))
                    .col(string(BankAccounts::Titular))
                    .col(string(BankAccounts::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_bank_accounts_workspace")
                            .from(BankAccounts::Table, BankAccounts::WorkspaceId)
                            .to(Workspace::Table, Workspace::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(BankAccounts::Table).to_owned())
            .await
    }
}

/// Fase 12 — núcleo de Finanças Gerais. Conta bancária de organização, mesmo
/// molde de `Custodia` (Fase 10 item 9): `Nome` (apelido livre, ex. "Nubank
/// PJ") e `Titular` (CPF/CNPJ, texto livre) em vez dos campos detalhados de
/// agência/conta do rascunho original — não usados em cálculo nenhum, então
/// não justificam colunas próprias ainda. Saldo é sempre derivado (soma de
/// `general_transactions`), nunca gravado aqui.
#[derive(DeriveIden)]
enum BankAccounts {
    Table,
    Id,
    WorkspaceId,
    Nome,
    Titular,
    CreatedAt,
}

/// Só `Table`/`Id` — esta migration não é dona do `workspace`, só aponta uma
/// foreign key pra ele.
#[derive(DeriveIden)]
enum Workspace {
    Table,
    Id,
}
