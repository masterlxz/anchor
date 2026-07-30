use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, Unchanged,
};
use serde::Deserialize;

use crate::entity::{asset_favorites, watchlist_items, watchlists};
use crate::error::AppError;

#[tauri::command]
pub async fn list_watchlists(
    db: tauri::State<'_, DatabaseConnection>,
    workspace_id: i32,
) -> Result<Vec<watchlists::Model>, AppError> {
    Ok(watchlists::Entity::find()
        .filter(watchlists::Column::WorkspaceId.eq(workspace_id))
        .all(db.inner())
        .await?)
}

#[derive(Deserialize)]
pub struct CreateWatchlistRequest {
    pub workspace_id: i32,
    pub name: String,
}

#[tauri::command]
pub async fn create_watchlist(
    db: tauri::State<'_, DatabaseConnection>,
    request: CreateWatchlistRequest,
) -> Result<watchlists::Model, AppError> {
    let watchlist = watchlists::ActiveModel {
        workspace_id: Set(request.workspace_id),
        name: Set(request.name),
        created_at: Set(chrono::Utc::now().to_rfc3339()),
        ..Default::default()
    }
    .insert(db.inner())
    .await?;

    Ok(watchlist)
}

#[tauri::command]
pub async fn delete_watchlist(
    db: tauri::State<'_, DatabaseConnection>,
    watchlist_id: i32,
) -> Result<(), AppError> {
    watchlists::Entity::delete_by_id(watchlist_id)
        .exec(db.inner())
        .await?;

    Ok(())
}

#[tauri::command]
pub async fn list_watchlist_items(
    db: tauri::State<'_, DatabaseConnection>,
    watchlist_id: i32,
) -> Result<Vec<watchlist_items::Model>, AppError> {
    Ok(watchlist_items::Entity::find()
        .filter(watchlist_items::Column::WatchlistId.eq(watchlist_id))
        .all(db.inner())
        .await?)
}

#[derive(Deserialize)]
pub struct AddWatchlistItemRequest {
    pub watchlist_id: i32,
    pub asset_id: i32,
    pub target_price: Option<f64>,
    pub notes: Option<String>,
}

#[tauri::command]
pub async fn add_watchlist_item(
    db: tauri::State<'_, DatabaseConnection>,
    request: AddWatchlistItemRequest,
) -> Result<watchlist_items::Model, AppError> {
    let item = watchlist_items::ActiveModel {
        watchlist_id: Set(request.watchlist_id),
        asset_id: Set(request.asset_id),
        target_price: Set(request.target_price),
        notes: Set(request.notes),
        created_at: Set(chrono::Utc::now().to_rfc3339()),
        ..Default::default()
    }
    .insert(db.inner())
    .await?;

    Ok(item)
}

#[derive(Deserialize)]
pub struct UpdateWatchlistItemRequest {
    pub item_id: i32,
    pub target_price: Option<f64>,
    pub notes: Option<String>,
}

#[tauri::command]
pub async fn update_watchlist_item(
    db: tauri::State<'_, DatabaseConnection>,
    request: UpdateWatchlistItemRequest,
) -> Result<watchlist_items::Model, AppError> {
    let existing = watchlist_items::Entity::find_by_id(request.item_id)
        .one(db.inner())
        .await?
        .ok_or_else(|| AppError::NotFound(format!("watchlist item {}", request.item_id)))?;

    let updated = watchlist_items::ActiveModel {
        id: Unchanged(existing.id),
        target_price: Set(request.target_price),
        notes: Set(request.notes),
        ..Default::default()
    }
    .update(db.inner())
    .await?;

    Ok(updated)
}

#[tauri::command]
pub async fn remove_watchlist_item(
    db: tauri::State<'_, DatabaseConnection>,
    item_id: i32,
) -> Result<(), AppError> {
    watchlist_items::Entity::delete_by_id(item_id)
        .exec(db.inner())
        .await?;

    Ok(())
}

#[tauri::command]
pub async fn list_favorite_assets(
    db: tauri::State<'_, DatabaseConnection>,
    workspace_id: i32,
) -> Result<Vec<asset_favorites::Model>, AppError> {
    Ok(asset_favorites::Entity::find()
        .filter(asset_favorites::Column::WorkspaceId.eq(workspace_id))
        .all(db.inner())
        .await?)
}

#[derive(Deserialize)]
pub struct ToggleFavoriteRequest {
    pub workspace_id: i32,
    pub asset_id: i32,
}

/// Um único comando pro botão de estrela: alterna entre favoritado/não em
/// vez de expor add/remove separados — a UI só precisa saber o `asset_id`,
/// sem rastrear o `id` da linha de favorito. Devolve o novo estado.
#[tauri::command]
pub async fn toggle_favorite(
    db: tauri::State<'_, DatabaseConnection>,
    request: ToggleFavoriteRequest,
) -> Result<bool, AppError> {
    let existing = asset_favorites::Entity::find()
        .filter(asset_favorites::Column::WorkspaceId.eq(request.workspace_id))
        .filter(asset_favorites::Column::AssetId.eq(request.asset_id))
        .one(db.inner())
        .await?;

    match existing {
        Some(favorite) => {
            asset_favorites::Entity::delete_by_id(favorite.id)
                .exec(db.inner())
                .await?;
            Ok(false)
        }
        None => {
            asset_favorites::ActiveModel {
                workspace_id: Set(request.workspace_id),
                asset_id: Set(request.asset_id),
                created_at: Set(chrono::Utc::now().to_rfc3339()),
                ..Default::default()
            }
            .insert(db.inner())
            .await?;
            Ok(true)
        }
    }
}
