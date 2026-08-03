use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use serde::{Deserialize, Serialize};

use super::valuation::ValuationOutcomeResponse;
use crate::domain::rnav::{self, RnavInputs};
use crate::entity::{rnav_inputs, valuation};
use crate::error::AppError;

#[derive(Deserialize)]
pub struct CalculateRnavRequest {
    pub ticker: String,
    pub reference_year: i32,
    pub current_price: f64,
    pub landbank: f64,
    pub inventory_at_market_value: f64,
    pub net_cash: f64,
    pub shares_outstanding: f64,
}

#[derive(Serialize)]
pub struct RnavValuationResponse {
    pub valuation: valuation::Model,
    pub inputs: rnav_inputs::Model,
}

// Sessão 56 — preview, sem persistir (ver `save_rnav` abaixo).
#[tauri::command]
pub async fn calculate_rnav(
    request: CalculateRnavRequest,
) -> Result<ValuationOutcomeResponse, AppError> {
    let outcome = rnav::calculate(
        &RnavInputs {
            landbank: request.landbank,
            inventory_at_market_value: request.inventory_at_market_value,
            net_cash: request.net_cash,
            shares_outstanding: request.shares_outstanding,
        },
        request.current_price,
    )?;

    Ok(ValuationOutcomeResponse {
        fair_price: outcome.fair_price,
        safety_margin: outcome.safety_margin,
        verdict: outcome.verdict.as_str().to_string(),
    })
}

#[tauri::command]
pub async fn save_rnav(
    db: tauri::State<'_, DatabaseConnection>,
    request: CalculateRnavRequest,
) -> Result<RnavValuationResponse, AppError> {
    let outcome = rnav::calculate(
        &RnavInputs {
            landbank: request.landbank,
            inventory_at_market_value: request.inventory_at_market_value,
            net_cash: request.net_cash,
            shares_outstanding: request.shares_outstanding,
        },
        request.current_price,
    )?;

    let valuation = valuation::ActiveModel {
        ticker: Set(request.ticker),
        reference_year: Set(request.reference_year),
        current_price: Set(request.current_price),
        model: Set("rnav".to_string()),
        fair_price: Set(Some(outcome.fair_price)),
        safety_margin: Set(Some(outcome.safety_margin)),
        verdict: Set(Some(outcome.verdict.as_str().to_string())),
        updated_at: Set(chrono::Utc::now().to_rfc3339()),
        ..Default::default()
    }
    .insert(db.inner())
    .await?;

    let inputs = rnav_inputs::ActiveModel {
        valuation_id: Set(valuation.id),
        landbank: Set(request.landbank),
        inventory_at_market_value: Set(request.inventory_at_market_value),
        net_cash: Set(request.net_cash),
        shares_outstanding: Set(request.shares_outstanding),
        ..Default::default()
    }
    .insert(db.inner())
    .await?;

    Ok(RnavValuationResponse { valuation, inputs })
}
