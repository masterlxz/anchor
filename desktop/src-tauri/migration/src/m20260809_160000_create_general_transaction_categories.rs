use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GeneralTransactionCategories::Table)
                    .if_not_exists()
                    .col(pk_auto(GeneralTransactionCategories::Id))
                    .col(integer(GeneralTransactionCategories::WorkspaceId))
                    .col(string(GeneralTransactionCategories::Nome))
                    .col(string(GeneralTransactionCategories::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_general_transaction_categories_workspace")
                            .from(
                                GeneralTransactionCategories::Table,
                                GeneralTransactionCategories::WorkspaceId,
                            )
                            .to(Workspace::Table, Workspace::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Três `alter_table` separados — SQLite/sea-query rejeita mais de um
        // `add_column`/`drop_column` numa `ALTER TABLE` só (mesmo achado de
        // `add_equity_fields_to_assets`). Sem FK declarada em `CategoryId`
        // (SQLite não suporta adicionar constraint de FK via `ALTER TABLE`,
        // só em `CREATE TABLE` — mesmo motivo pelo qual nenhuma outra coluna
        // de FK deste projeto foi adicionada assim; a relação `belongs_to`
        // do SeaORM funciona no nível do ORM independente de constraint no
        // banco).
        manager
            .alter_table(
                Table::alter()
                    .table(GeneralTransactions::Table)
                    .add_column(integer_null(GeneralTransactions::CategoryId))
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(GeneralTransactions::Table)
                    .drop_column(GeneralTransactions::Categoria)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(GeneralTransactions::Table)
                    .add_column(string_null(GeneralTransactions::Categoria))
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(GeneralTransactions::Table)
                    .drop_column(GeneralTransactions::CategoryId)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(GeneralTransactionCategories::Table)
                    .to_owned(),
            )
            .await
    }
}

/// Fase 12 — categoria de lançamento vira entidade própria (Sessão 71,
/// feedback do dono do projeto depois do teste ao vivo do núcleo): antes,
/// `categoria` era texto livre digitado a cada lançamento; agora é
/// cadastrada uma vez e só selecionada na hora do lançamento. Sem limite de
/// orçamento mensal ainda — `Category` do rascunho original tinha esse
/// campo, decisão explícita de deixar de fora nesta fatia (só nome).
#[derive(DeriveIden)]
enum GeneralTransactionCategories {
    Table,
    Id,
    WorkspaceId,
    Nome,
    CreatedAt,
}

#[derive(DeriveIden)]
enum GeneralTransactions {
    Table,
    Categoria,
    CategoryId,
}

/// Só `Table`/`Id` — esta migration não é dona do `workspace`, só aponta
/// uma foreign key pra ele.
#[derive(DeriveIden)]
enum Workspace {
    Table,
    Id,
}
