use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use serde::{Deserialize, Serialize};

use super::valuation::ValuationOutcomeResponse;
use crate::domain::rim::{self, RimInputs};
use crate::entity::{rim_inputs, valuation};
use crate::error::AppError;

#[derive(Deserialize)]
pub struct CalculateRimRequest {
    pub ticker: String,
    pub reference_year: i32,
    pub current_price: f64,
    pub book_value_per_share: f64,
    pub roe_current: f64,
    pub payout: f64,
    pub ke: f64,
    pub fade_years: i32,
}

#[derive(Serialize)]
pub struct RimValuationResponse {
    pub valuation: valuation::Model,
    pub inputs: rim_inputs::Model,
}

// Sessão 56 — preview, sem persistir (ver `save_rim` abaixo).
#[tauri::command]
pub async fn calculate_rim(
    request: CalculateRimRequest,
) -> Result<ValuationOutcomeResponse, AppError> {
    let outcome = rim::calculate(
        &RimInputs {
            book_value_per_share: request.book_value_per_share,
            roe_current: request.roe_current,
            payout: request.payout,
            ke: request.ke,
            fade_years: request.fade_years,
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
pub async fn save_rim(
    db: tauri::State<'_, DatabaseConnection>,
    request: CalculateRimRequest,
) -> Result<RimValuationResponse, AppError> {
    let outcome = rim::calculate(
        &RimInputs {
            book_value_per_share: request.book_value_per_share,
            roe_current: request.roe_current,
            payout: request.payout,
            ke: request.ke,
            fade_years: request.fade_years,
        },
        request.current_price,
    )?;

    let valuation = valuation::ActiveModel {
        ticker: Set(request.ticker),
        reference_year: Set(request.reference_year),
        current_price: Set(request.current_price),
        model: Set("rim".to_string()),
        fair_price: Set(Some(outcome.fair_price)),
        safety_margin: Set(Some(outcome.safety_margin)),
        verdict: Set(Some(outcome.verdict.as_str().to_string())),
        updated_at: Set(chrono::Utc::now().to_rfc3339()),
        ..Default::default()
    }
    .insert(db.inner())
    .await?;

    let inputs = rim_inputs::ActiveModel {
        valuation_id: Set(valuation.id),
        book_value_per_share: Set(request.book_value_per_share),
        roe_current: Set(request.roe_current),
        payout: Set(request.payout),
        ke: Set(request.ke),
        fade_years: Set(request.fade_years),
        ..Default::default()
    }
    .insert(db.inner())
    .await?;

    Ok(RimValuationResponse { valuation, inputs })
}
