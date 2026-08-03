use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use serde::{Deserialize, Serialize};

use super::valuation::ValuationOutcomeResponse;
use crate::domain::dcf::{self, DcfInputs};
use crate::entity::{dcf_inputs, valuation};
use crate::error::AppError;

#[derive(Deserialize)]
pub struct CalculateDcfRequest {
    pub ticker: String,
    pub reference_year: i32,
    pub current_price: f64,
    pub ebit: f64,
    pub tax_rate: f64,
    pub depreciation_amortization: f64,
    pub capex: f64,
    pub nwc_change: f64,
    pub total_debt: f64,
    pub cash: f64,
    pub shares_outstanding: f64,
    pub beta: f64,
    pub risk_free_rate: f64,
    pub market_risk_premium: f64,
    pub kd: f64,
    pub perpetuity_growth: f64,
}

#[derive(Serialize)]
pub struct DcfValuationResponse {
    pub valuation: valuation::Model,
    pub inputs: dcf_inputs::Model,
}

// Sessão 56 — preview, sem persistir (ver `save_dcf` abaixo).
#[tauri::command]
pub async fn calculate_dcf(
    request: CalculateDcfRequest,
) -> Result<ValuationOutcomeResponse, AppError> {
    let outcome = dcf::calculate(
        &DcfInputs {
            ebit: request.ebit,
            tax_rate: request.tax_rate,
            depreciation_amortization: request.depreciation_amortization,
            capex: request.capex,
            nwc_change: request.nwc_change,
            total_debt: request.total_debt,
            cash: request.cash,
            shares_outstanding: request.shares_outstanding,
            beta: request.beta,
            risk_free_rate: request.risk_free_rate,
            market_risk_premium: request.market_risk_premium,
            kd: request.kd,
            perpetuity_growth: request.perpetuity_growth,
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
pub async fn save_dcf(
    db: tauri::State<'_, DatabaseConnection>,
    request: CalculateDcfRequest,
) -> Result<DcfValuationResponse, AppError> {
    let outcome = dcf::calculate(
        &DcfInputs {
            ebit: request.ebit,
            tax_rate: request.tax_rate,
            depreciation_amortization: request.depreciation_amortization,
            capex: request.capex,
            nwc_change: request.nwc_change,
            total_debt: request.total_debt,
            cash: request.cash,
            shares_outstanding: request.shares_outstanding,
            beta: request.beta,
            risk_free_rate: request.risk_free_rate,
            market_risk_premium: request.market_risk_premium,
            kd: request.kd,
            perpetuity_growth: request.perpetuity_growth,
        },
        request.current_price,
    )?;

    let valuation = valuation::ActiveModel {
        ticker: Set(request.ticker),
        reference_year: Set(request.reference_year),
        current_price: Set(request.current_price),
        model: Set("dcf".to_string()),
        fair_price: Set(Some(outcome.fair_price)),
        safety_margin: Set(Some(outcome.safety_margin)),
        verdict: Set(Some(outcome.verdict.as_str().to_string())),
        updated_at: Set(chrono::Utc::now().to_rfc3339()),
        ..Default::default()
    }
    .insert(db.inner())
    .await?;

    let inputs = dcf_inputs::ActiveModel {
        valuation_id: Set(valuation.id),
        ebit: Set(request.ebit),
        tax_rate: Set(request.tax_rate),
        depreciation_amortization: Set(request.depreciation_amortization),
        capex: Set(request.capex),
        nwc_change: Set(request.nwc_change),
        total_debt: Set(request.total_debt),
        cash: Set(request.cash),
        shares_outstanding: Set(request.shares_outstanding),
        beta: Set(request.beta),
        risk_free_rate: Set(request.risk_free_rate),
        market_risk_premium: Set(request.market_risk_premium),
        kd: Set(request.kd),
        perpetuity_growth: Set(request.perpetuity_growth),
        ..Default::default()
    }
    .insert(db.inner())
    .await?;

    Ok(DcfValuationResponse { valuation, inputs })
}
