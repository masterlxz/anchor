use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, Unchanged,
};
use serde::Deserialize;

use crate::entity::portfolio;
use crate::error::AppError;

#[tauri::command]
pub async fn list_portfolios(
    db: tauri::State<'_, DatabaseConnection>,
    workspace_id: i32,
) -> Result<Vec<portfolio::Model>, AppError> {
    Ok(portfolio::Entity::find()
        .filter(portfolio::Column::WorkspaceId.eq(workspace_id))
        .all(db.inner())
        .await?)
}

#[derive(Deserialize)]
pub struct CreatePortfolioRequest {
    pub workspace_id: i32,
    pub name: String,
    pub description: Option<String>,
}

#[tauri::command]
pub async fn create_portfolio(
    db: tauri::State<'_, DatabaseConnection>,
    request: CreatePortfolioRequest,
) -> Result<portfolio::Model, AppError> {
    let portfolio = portfolio::ActiveModel {
        workspace_id: Set(request.workspace_id),
        name: Set(request.name),
        description: Set(request.description),
        created_at: Set(chrono::Utc::now().to_rfc3339()),
        ..Default::default()
    }
    .insert(db.inner())
    .await?;

    Ok(portfolio)
}

#[derive(Deserialize)]
pub struct RenamePortfolioRequest {
    pub portfolio_id: i32,
    pub name: String,
}

#[tauri::command]
pub async fn rename_portfolio(
    db: tauri::State<'_, DatabaseConnection>,
    request: RenamePortfolioRequest,
) -> Result<portfolio::Model, AppError> {
    let existing = portfolio::Entity::find_by_id(request.portfolio_id)
        .one(db.inner())
        .await?
        .ok_or_else(|| AppError::NotFound(format!("portfolio {}", request.portfolio_id)))?;

    let updated = portfolio::ActiveModel {
        id: Unchanged(existing.id),
        name: Set(request.name),
        ..Default::default()
    }
    .update(db.inner())
    .await?;

    Ok(updated)
}
