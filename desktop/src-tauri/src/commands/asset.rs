use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, QueryOrder, Set};
use serde::Deserialize;

use crate::entity::assets;
use crate::error::AppError;

// Fase 10.2, escopo Sessão 29 — as classes expandidas na Sessão 30 (FII,
// REIT, ETF, cripto, metal, imóvel, empresa não listada, ver PHASE.md item
// 8) ficam pra uma fatia futura, junto com as decisões em aberto delas.
const ASSET_CLASSES: [&str; 4] = [
    "acao_br",
    "acao_internacional",
    "tesouro_direto",
    "renda_fixa",
];
const EXPOSURE_TYPES: [&str; 2] = ["pais", "categoria_especial"];

#[tauri::command]
pub async fn list_assets(
    db: tauri::State<'_, DatabaseConnection>,
) -> Result<Vec<assets::Model>, AppError> {
    Ok(assets::Entity::find()
        .order_by_asc(assets::Column::Ticker)
        .all(db.inner())
        .await?)
}

#[derive(Deserialize)]
pub struct CreateAssetRequest {
    pub ticker: String,
    pub name: String,
    pub asset_class: String,
    pub currency: String,
    pub exchange: Option<String>,
    pub exposure_type: String,
    pub exposure_value: String,
}

#[tauri::command]
pub async fn create_asset(
    db: tauri::State<'_, DatabaseConnection>,
    request: CreateAssetRequest,
) -> Result<assets::Model, AppError> {
    if !ASSET_CLASSES.contains(&request.asset_class.as_str()) {
        return Err(AppError::InvalidGuard(format!(
            "asset_class '{}' not supported yet (expected one of {ASSET_CLASSES:?})",
            request.asset_class
        )));
    }
    if !EXPOSURE_TYPES.contains(&request.exposure_type.as_str()) {
        return Err(AppError::InvalidGuard(format!(
            "exposure_type '{}' invalid (expected 'pais' or 'categoria_especial')",
            request.exposure_type
        )));
    }

    let asset = assets::ActiveModel {
        ticker: Set(request.ticker),
        name: Set(request.name),
        asset_class: Set(request.asset_class),
        currency: Set(request.currency),
        exchange: Set(request.exchange),
        exposure_type: Set(request.exposure_type),
        exposure_value: Set(request.exposure_value),
        created_at: Set(chrono::Utc::now().to_rfc3339()),
        ..Default::default()
    }
    .insert(db.inner())
    .await?;

    Ok(asset)
}
