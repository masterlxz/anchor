use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Custodia::Table)
                    .if_not_exists()
                    .col(pk_auto(Custodia::Id))
                    .col(integer(Custodia::WorkspaceId))
                    .col(string(Custodia::Instituicao))
                    .col(string(Custodia::Titular))
                    .col(string(Custodia::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_custodia_workspace")
                            .from(Custodia::Table, Custodia::WorkspaceId)
                            .to(Workspace::Table, Workspace::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Custodia::Table).to_owned())
            .await
    }
}

/// Fase 10, item 9 — onde um ativo está guardado (corretora/exchange/wallet),
/// ortogonal à classe do ativo. Cadastro livre, sem lista pré-pronta.
/// `Instituicao` (ex.: "Rico", "Ledger") e `Titular` (CPF/CNPJ) são dois
/// campos separados na mesma entidade, permitindo agrupar por qualquer um
/// dos dois sem hierarquia extra (ver PHASE.md, item 9).
#[derive(DeriveIden)]
enum Custodia {
    Table,
    Id,
    WorkspaceId,
    Instituicao,
    Titular,
    CreatedAt,
}

/// Só `Table`/`Id` — esta migration não é dona do `workspace` (criado em
/// `m20260729_090000_...`), só aponta uma foreign key pra ele.
#[derive(DeriveIden)]
enum Workspace {
    Table,
    Id,
}
