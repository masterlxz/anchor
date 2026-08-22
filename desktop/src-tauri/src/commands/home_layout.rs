// Fase 10.8 fatia 2 — persistência do layout personalizado (ordem/
// tamanho/visibilidade) dos widgets da Home do Workspace. Uma linha por
// widget por workspace (`home_widget_layout`, migration
// `m20260822_100000`); `save_home_layout` substitui o conjunto inteiro em
// vez de fazer diff, mesmo raciocínio simples de "replace" já usado em
// `liability.rs` pras parcelas (delete_many seguido de insert). Lista vazia
// de `get_home_layout` é estado válido (nunca customizado) — o frontend cai
// pro layout padrão hardcoded nesse caso.

use chrono::Utc;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::Deserialize;

use crate::entity::home_widget_layout;
use crate::error::AppError;

#[derive(Deserialize)]
pub struct HomeWidgetLayoutInput {
    pub widget_key: String,
    pub position: i32,
    pub w: i32,
    pub h: i32,
    pub visible: bool,
}

#[tauri::command]
pub async fn get_home_layout(
    db: tauri::State<'_, DatabaseConnection>,
    workspace_id: i32,
) -> Result<Vec<home_widget_layout::Model>, AppError> {
    let db = db.inner();
    let rows = home_widget_layout::Entity::find()
        .filter(home_widget_layout::Column::WorkspaceId.eq(workspace_id))
        .all(db)
        .await?;
    Ok(rows)
}

#[tauri::command]
pub async fn save_home_layout(
    db: tauri::State<'_, DatabaseConnection>,
    workspace_id: i32,
    widgets: Vec<HomeWidgetLayoutInput>,
) -> Result<(), AppError> {
    let db = db.inner();

    home_widget_layout::Entity::delete_many()
        .filter(home_widget_layout::Column::WorkspaceId.eq(workspace_id))
        .exec(db)
        .await?;

    if widgets.is_empty() {
        return Ok(());
    }

    let now = Utc::now().to_rfc3339();
    let models: Vec<home_widget_layout::ActiveModel> = widgets
        .into_iter()
        .map(|widget| home_widget_layout::ActiveModel {
            id: sea_orm::ActiveValue::NotSet,
            workspace_id: Set(workspace_id),
            widget_key: Set(widget.widget_key),
            position: Set(widget.position),
            w: Set(widget.w),
            h: Set(widget.h),
            visible: Set(widget.visible),
            updated_at: Set(now.clone()),
        })
        .collect();

    home_widget_layout::Entity::insert_many(models).exec(db).await?;

    Ok(())
}
