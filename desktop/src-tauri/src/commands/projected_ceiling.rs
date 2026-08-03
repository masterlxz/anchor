use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use serde::{Deserialize, Serialize};

use super::valuation::ValuationOutcomeResponse;
use crate::domain::projected_ceiling::{self, ProjectedCeilingInputs};
use crate::entity::{projected_ceiling_inputs, valuation};
use crate::error::AppError;

#[derive(Deserialize)]
pub struct CalculateProjectedCeilingRequest {
    pub ticker: String,
    pub reference_year: i32,
    pub current_price: f64,
    pub current_dividend: f64,
    pub expected_growth: f64,
    pub projection_years: i32,
    pub desired_yield: f64,
    pub ke: f64,
}

#[derive(Serialize)]
pub struct ProjectedCeilingValuationResponse {
    pub valuation: valuation::Model,
    pub inputs: projected_ceiling_inputs::Model,
}

// Sessão 56 — preview, sem persistir (ver `save_projected_ceiling` abaixo).
#[tauri::command]
pub async fn calculate_projected_ceiling(
    request: CalculateProjectedCeilingRequest,
) -> Result<ValuationOutcomeResponse, AppError> {
    let outcome = projected_ceiling::calculate(
        &ProjectedCeilingInputs {
            current_dividend: request.current_dividend,
            expected_growth: request.expected_growth,
            projection_years: request.projection_years,
            desired_yield: request.desired_yield,
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
pub async fn save_projected_ceiling(
    db: tauri::State<'_, DatabaseConnection>,
    request: CalculateProjectedCeilingRequest,
) -> Result<ProjectedCeilingValuationResponse, AppError> {
    let outcome = projected_ceiling::calculate(
        &ProjectedCeilingInputs {
            current_dividend: request.current_dividend,
            expected_growth: request.expected_growth,
            projection_years: request.projection_years,
            desired_yield: request.desired_yield,
            ke: request.ke,
        },
        request.current_price,
    )?;

    let valuation = valuation::ActiveModel {
        ticker: Set(request.ticker),
        reference_year: Set(request.reference_year),
        current_price: Set(request.current_price),
        model: Set("projected_ceiling".to_string()),
        fair_price: Set(Some(outcome.fair_price)),
        safety_margin: Set(Some(outcome.safety_margin)),
        verdict: Set(Some(outcome.verdict.as_str().to_string())),
        updated_at: Set(chrono::Utc::now().to_rfc3339()),
        ..Default::default()
    }
    .insert(db.inner())
    .await?;

    let inputs = projected_ceiling_inputs::ActiveModel {
        valuation_id: Set(valuation.id),
        current_dividend: Set(request.current_dividend),
        expected_growth: Set(request.expected_growth),
        projection_years: Set(request.projection_years),
        desired_yield: Set(request.desired_yield),
        ke: Set(request.ke),
        ..Default::default()
    }
    .insert(db.inner())
    .await?;

    Ok(ProjectedCeilingValuationResponse { valuation, inputs })
}
