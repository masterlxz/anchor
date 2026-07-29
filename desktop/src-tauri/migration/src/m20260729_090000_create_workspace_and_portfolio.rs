use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Workspace::Table)
                    .if_not_exists()
                    .col(pk_auto(Workspace::Id))
                    .col(string(Workspace::Name))
                    .col(string(Workspace::CreatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Portfolio::Table)
                    .if_not_exists()
                    .col(pk_auto(Portfolio::Id))
                    .col(integer(Portfolio::WorkspaceId))
                    .col(string(Portfolio::Name))
                    .col(string_null(Portfolio::Description))
                    .col(string(Portfolio::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_portfolio_workspace")
                            .from(Portfolio::Table, Portfolio::WorkspaceId)
                            .to(Workspace::Table, Workspace::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Fase 10.1: Workspace nasce single-user (sem WorkspaceMember/convite
        // ainda, ver PHASE.md item 10.1) — em vez de resolver "qual workspace
        // é o do usuário" em toda query, semeia exatamente uma linha aqui,
        // mesmo espírito do único registro implícito que `ai_api_key` já
        // assume (nenhuma coluna de dono, o banco inteiro é de um usuário só).
        // Uma Portfolio padrão também é semeada para não deixar o usuário
        // numa tela vazia sem conseguir lançar nada ainda.
        let now = chrono::Utc::now().to_rfc3339();
        let db = manager.get_connection();
        db.execute_unprepared(&format!(
            "INSERT INTO workspace (name, created_at) VALUES ('Meu Workspace', '{now}')"
        ))
        .await?;
        db.execute_unprepared(&format!(
            "INSERT INTO portfolio (workspace_id, name, description, created_at) VALUES (1, 'Carteira Principal', NULL, '{now}')"
        ))
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Portfolio::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Workspace::Table).to_owned())
            .await
    }
}

/// Fase 10.1 — single-user "workspace" (ver PHASE.md, item 1): hoje é sempre
/// exatamente uma linha, semeada acima. `WorkspaceMember`/RBAC (item 10.7)
/// fica bloqueado até a permissão descentralizada via TruthID+IPFS (Fase 8)
/// estar resolvida.
#[derive(DeriveIden)]
enum Workspace {
    Table,
    Id,
    Name,
    CreatedAt,
}

/// Uma carteira dentro de um Workspace (Fase 10, item 2) — pertence ao
/// Workspace, não a um membro individual (decisão explícita da Sessão 29).
#[derive(DeriveIden)]
enum Portfolio {
    Table,
    Id,
    WorkspaceId,
    Name,
    Description,
    CreatedAt,
}
