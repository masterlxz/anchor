use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Watchlists::Table)
                    .if_not_exists()
                    .col(pk_auto(Watchlists::Id))
                    .col(integer(Watchlists::WorkspaceId))
                    .col(string(Watchlists::Name))
                    .col(string(Watchlists::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_watchlists_workspace")
                            .from(Watchlists::Table, Watchlists::WorkspaceId)
                            .to(Workspace::Table, Workspace::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(WatchlistItems::Table)
                    .if_not_exists()
                    .col(pk_auto(WatchlistItems::Id))
                    .col(integer(WatchlistItems::WatchlistId))
                    .col(integer(WatchlistItems::AssetId))
                    .col(double_null(WatchlistItems::TargetPrice))
                    .col(string_null(WatchlistItems::Notes))
                    .col(string(WatchlistItems::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_watchlist_items_watchlist")
                            .from(WatchlistItems::Table, WatchlistItems::WatchlistId)
                            .to(Watchlists::Table, Watchlists::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    // Sem `on_delete` pra Asset, mesmo raciocínio de
                    // `transactions`/Fase 10.2: apagar um cadastro de
                    // referência não deveria arrastar a anotação/preço-alvo.
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_watchlist_items_asset")
                            .from(WatchlistItems::Table, WatchlistItems::AssetId)
                            .to(Assets::Table, Assets::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AssetFavorites::Table)
                    .if_not_exists()
                    .col(pk_auto(AssetFavorites::Id))
                    .col(integer(AssetFavorites::WorkspaceId))
                    .col(integer(AssetFavorites::AssetId))
                    .col(string(AssetFavorites::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_asset_favorites_workspace")
                            .from(AssetFavorites::Table, AssetFavorites::WorkspaceId)
                            .to(Workspace::Table, Workspace::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_asset_favorites_asset")
                            .from(AssetFavorites::Table, AssetFavorites::AssetId)
                            .to(Assets::Table, Assets::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_asset_favorites_workspace_asset")
                    .table(AssetFavorites::Table)
                    .col(AssetFavorites::WorkspaceId)
                    .col(AssetFavorites::AssetId)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AssetFavorites::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WatchlistItems::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Watchlists::Table).to_owned())
            .await
    }
}

/// Fase 10.4 — listas nomeadas de ativos (ex.: "Ações de Dividendos"), com
/// preço-alvo/notas por item (ver PHASE.md item 5). Pertence ao Workspace,
/// mesma hierarquia de posse de `Portfolio`/`Custodia` (Cascade).
#[derive(DeriveIden)]
enum Watchlists {
    Table,
    Id,
    WorkspaceId,
    Name,
    CreatedAt,
}

/// Item de uma watchlist — morre com a lista (Cascade), aponta pro catálogo
/// global de `Assets` sem arrastar exclusão (mesmo raciocínio de
/// `transactions`).
#[derive(DeriveIden)]
enum WatchlistItems {
    Table,
    Id,
    WatchlistId,
    AssetId,
    TargetPrice,
    Notes,
    CreatedAt,
}

/// Fase 10.4 — mecanismo separado de favorito rápido (estrela), distinto de
/// watchlist nomeada por pedido explícito do dono do projeto (ver
/// AskUserQuestion da Sessão 36): um clique em qualquer ativo, sem precisar
/// abrir uma lista. Índice único `(workspace_id, asset_id)` garante que
/// `toggle_favorite` nunca duplica a mesma linha.
#[derive(DeriveIden)]
enum AssetFavorites {
    Table,
    Id,
    WorkspaceId,
    AssetId,
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
enum Assets {
    Table,
    Id,
}
