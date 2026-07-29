use sea_orm::{DatabaseConnection, EntityTrait};

use crate::entity::workspace;
use crate::error::AppError;

/// Fase 10.1 — o app hoje sempre tem exatamente um Workspace, semeado pela
/// migration `m20260729_090000_...` (sem `WorkspaceMember`/convite ainda,
/// ver PHASE.md item 10.1). Sem parâmetro: não há "qual workspace é o meu"
/// pra resolver enquanto isso for verdade.
#[tauri::command]
pub async fn get_workspace(
    db: tauri::State<'_, DatabaseConnection>,
) -> Result<workspace::Model, AppError> {
    workspace::Entity::find()
        .one(db.inner())
        .await?
        .ok_or_else(|| AppError::NotFound("workspace".to_string()))
}
