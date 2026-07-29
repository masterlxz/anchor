use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::Deserialize;

use crate::entity::custodia;
use crate::error::AppError;

#[tauri::command]
pub async fn list_custodias(
    db: tauri::State<'_, DatabaseConnection>,
    workspace_id: i32,
) -> Result<Vec<custodia::Model>, AppError> {
    Ok(custodia::Entity::find()
        .filter(custodia::Column::WorkspaceId.eq(workspace_id))
        .all(db.inner())
        .await?)
}

#[derive(Deserialize)]
pub struct CreateCustodiaRequest {
    pub workspace_id: i32,
    pub instituicao: String,
    pub titular: String,
}

#[tauri::command]
pub async fn create_custodia(
    db: tauri::State<'_, DatabaseConnection>,
    request: CreateCustodiaRequest,
) -> Result<custodia::Model, AppError> {
    let custodia = custodia::ActiveModel {
        workspace_id: Set(request.workspace_id),
        instituicao: Set(request.instituicao),
        titular: Set(request.titular),
        created_at: Set(chrono::Utc::now().to_rfc3339()),
        ..Default::default()
    }
    .insert(db.inner())
    .await?;

    Ok(custodia)
}

#[tauri::command]
pub async fn delete_custodia(
    db: tauri::State<'_, DatabaseConnection>,
    custodia_id: i32,
) -> Result<(), AppError> {
    custodia::Entity::delete_by_id(custodia_id)
        .exec(db.inner())
        .await?;

    Ok(())
}
