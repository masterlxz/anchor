use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::Deserialize;

use crate::entity::general_transaction_category;
use crate::error::AppError;

#[derive(Deserialize)]
pub struct CreateGeneralTransactionCategoryRequest {
    pub workspace_id: i32,
    pub nome: String,
}

#[tauri::command]
pub async fn create_general_transaction_category(
    db: tauri::State<'_, DatabaseConnection>,
    request: CreateGeneralTransactionCategoryRequest,
) -> Result<general_transaction_category::Model, AppError> {
    let category = general_transaction_category::ActiveModel {
        workspace_id: Set(request.workspace_id),
        nome: Set(request.nome),
        created_at: Set(chrono::Utc::now().to_rfc3339()),
        ..Default::default()
    }
    .insert(db.inner())
    .await?;

    Ok(category)
}

#[tauri::command]
pub async fn list_general_transaction_categories(
    db: tauri::State<'_, DatabaseConnection>,
    workspace_id: i32,
) -> Result<Vec<general_transaction_category::Model>, AppError> {
    Ok(general_transaction_category::Entity::find()
        .filter(general_transaction_category::Column::WorkspaceId.eq(workspace_id))
        .all(db.inner())
        .await?)
}

#[tauri::command]
pub async fn delete_general_transaction_category(
    db: tauri::State<'_, DatabaseConnection>,
    general_transaction_category_id: i32,
) -> Result<(), AppError> {
    general_transaction_category::Entity::delete_by_id(general_transaction_category_id)
        .exec(db.inner())
        .await?;

    Ok(())
}
