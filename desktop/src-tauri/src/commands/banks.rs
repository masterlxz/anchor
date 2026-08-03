use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use serde::{Deserialize, Serialize};

use super::valuation::ValuationOutcomeResponse;
use crate::domain::banks::{self, BanksInputs};
use crate::entity::{banks_inputs, valuation};
use crate::error::AppError;

#[derive(Deserialize)]
pub struct CalculateBanksRequest {
    pub ticker: String,
    pub reference_year: i32,
    pub current_price: f64,
    pub book_value_per_share: f64,
    pub roe: f64,
    pub payout: f64,
    pub ke: f64,
}

#[derive(Serialize)]
pub struct BanksValuationResponse {
    pub valuation: valuation::Model,
    pub inputs: banks_inputs::Model,
}

// Sessão 56 — preview, sem persistir (ver `save_banks` abaixo).
#[tauri::command]
pub async fn calculate_banks(
    request: CalculateBanksRequest,
) -> Result<ValuationOutcomeResponse, AppError> {
    let outcome = banks::calculate(
        &BanksInputs {
            book_value_per_share: request.book_value_per_share,
            roe: request.roe,
            payout: request.payout,
            ke: request.ke,
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
pub async fn save_banks(
    db: tauri::State<'_, DatabaseConnection>,
    request: CalculateBanksRequest,
) -> Result<BanksValuationResponse, AppError> {
    let outcome = banks::calculate(
        &BanksInputs {
            book_value_per_share: request.book_value_per_share,
            roe: request.roe,
            payout: request.payout,
            ke: request.ke,
        },
        request.current_price,
    )?;

    let valuation = valuation::ActiveModel {
        ticker: Set(request.ticker),
        reference_year: Set(request.reference_year),
        current_price: Set(request.current_price),
        model: Set("banks".to_string()),
        fair_price: Set(Some(outcome.fair_price)),
        safety_margin: Set(Some(outcome.safety_margin)),
        verdict: Set(Some(outcome.verdict.as_str().to_string())),
        updated_at: Set(chrono::Utc::now().to_rfc3339()),
        ..Default::default()
    }
    .insert(db.inner())
    .await?;

    let inputs = banks_inputs::ActiveModel {
        valuation_id: Set(valuation.id),
        book_value_per_share: Set(request.book_value_per_share),
        roe: Set(request.roe),
        payout: Set(request.payout),
        ke: Set(request.ke),
        ..Default::default()
    }
    .insert(db.inner())
    .await?;

    Ok(BanksValuationResponse { valuation, inputs })
}
