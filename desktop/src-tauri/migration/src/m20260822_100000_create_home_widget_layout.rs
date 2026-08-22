use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(HomeWidgetLayout::Table)
                    .if_not_exists()
                    .col(pk_auto(HomeWidgetLayout::Id))
                    .col(integer(HomeWidgetLayout::WorkspaceId))
                    .col(string(HomeWidgetLayout::WidgetKey))
                    .col(integer(HomeWidgetLayout::Position))
                    .col(integer(HomeWidgetLayout::W))
                    .col(integer(HomeWidgetLayout::H))
                    .col(boolean(HomeWidgetLayout::Visible))
                    .col(string(HomeWidgetLayout::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_home_widget_layout_workspace")
                            .from(HomeWidgetLayout::Table, HomeWidgetLayout::WorkspaceId)
                            .to(Workspace::Table, Workspace::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_home_widget_layout_workspace_widget")
                    .table(HomeWidgetLayout::Table)
                    .col(HomeWidgetLayout::WorkspaceId)
                    .col(HomeWidgetLayout::WidgetKey)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(HomeWidgetLayout::Table).to_owned())
            .await
    }
}

/// Fase 10.8 fatia 2 — ordem/tamanho/visibilidade de cada widget da Home do
/// Workspace, salvos por `save_home_layout` quando o dono do projeto clica
/// "Save" no modo "Customize layout" (`commands::home_layout`). Uma linha
/// por widget por workspace; `save_home_layout` substitui o conjunto
/// inteiro (delete + insert) em vez de fazer diff. Lista vazia é estado
/// válido (nunca customizado) — o frontend cai pro layout padrão hardcoded.
/// `position` é o índice na lista reordenável (dnd-kit) — sem `x`/`y`
/// absolutos porque a v1 com `react-grid-layout` (posicionamento livre via
/// x/y + CSS transform) travava o WebKitGTK deste projeto em tela preta;
/// trocado por reordenar+redimensionar em fluxo normal de CSS grid.
#[derive(DeriveIden)]
enum HomeWidgetLayout {
    Table,
    Id,
    WorkspaceId,
    WidgetKey,
    Position,
    W,
    H,
    Visible,
    UpdatedAt,
}

/// Só `Table`/`Id` — aponta FK pra tabela dona, criada em migration anterior.
#[derive(DeriveIden)]
enum Workspace {
    Table,
    Id,
}
