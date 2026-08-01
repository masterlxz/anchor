use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde::Deserialize;

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

// Fase 10.6 — múltiplos workspaces locais (sem dono/convite ainda, ver
// PHASE.md item 10.6). `list_workspaces`/`create_workspace` alimentam a
// tela de entrada do app, onde o usuário escolhe ou cria um workspace antes
// de ver as abas normais.

#[tauri::command]
pub async fn list_workspaces(
    db: tauri::State<'_, DatabaseConnection>,
) -> Result<Vec<workspace::Model>, AppError> {
    Ok(workspace::Entity::find().all(db.inner()).await?)
}

#[derive(Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
}

#[tauri::command]
pub async fn create_workspace(
    db: tauri::State<'_, DatabaseConnection>,
    request: CreateWorkspaceRequest,
) -> Result<workspace::Model, AppError> {
    let workspace = workspace::ActiveModel {
        name: Set(request.name),
        created_at: Set(chrono::Utc::now().to_rfc3339()),
        ..Default::default()
    }
    .insert(db.inner())
    .await?;

    Ok(workspace)
}
