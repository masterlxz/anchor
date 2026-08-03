use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use serde::{Deserialize, Serialize};

use super::valuation::ValuationOutcomeResponse;
use crate::domain::bazin::{self, BazinInputs};
use crate::entity::{bazin_inputs, valuation};
use crate::error::AppError;

#[derive(Deserialize)]
pub struct CalculateBazinRequest {
    pub ticker: String,
    pub reference_year: i32,
    pub current_price: f64,
    pub average_dividend: f64,
    pub desired_yield: f64,
}

#[derive(Serialize)]
pub struct BazinValuationResponse {
    pub valuation: valuation::Model,
    pub inputs: bazin_inputs::Model,
}

// Sessão 56 — separado de `save_bazin` (que persiste): só roda o domínio e
// devolve o resultado, sem tocar no banco. Pedido explícito do dono do
// projeto: "Calculate" vira preview, "Save" vira o botão que grava de
// verdade — antes eram a mesma chamada.
#[tauri::command]
pub async fn calculate_bazin(
    request: CalculateBazinRequest,
) -> Result<ValuationOutcomeResponse, AppError> {
    let outcome = bazin::calculate(
        &BazinInputs {
            average_dividend: request.average_dividend,
            desired_yield: request.desired_yield,
        },
        request.current_price,
    )?;

    Ok(ValuationOutcomeResponse {
        fair_price: outcome.fair_price,
        safety_margin: outcome.safety_margin,
        verdict: outcome.verdict.as_str().to_string(),
    })
}

// O que `calculate_bazin` fazia antes da Sessão 56 (calcula + insere) —
// recalcula do zero em vez de confiar num resultado já computado no
// cliente, mesmo princípio de não confiar em número silencioso já usado no
// resto do projeto.
#[tauri::command]
pub async fn save_bazin(
    db: tauri::State<'_, DatabaseConnection>,
    request: CalculateBazinRequest,
) -> Result<BazinValuationResponse, AppError> {
    let outcome = bazin::calculate(
        &BazinInputs {
            average_dividend: request.average_dividend,
            desired_yield: request.desired_yield,
        },
        request.current_price,
    )?;

    let valuation = valuation::ActiveModel {
        ticker: Set(request.ticker),
        reference_year: Set(request.reference_year),
        current_price: Set(request.current_price),
        model: Set("bazin".to_string()),
        fair_price: Set(Some(outcome.fair_price)),
        safety_margin: Set(Some(outcome.safety_margin)),
        verdict: Set(Some(outcome.verdict.as_str().to_string())),
        updated_at: Set(chrono::Utc::now().to_rfc3339()),
        ..Default::default()
    }
    .insert(db.inner())
    .await?;

    let inputs = bazin_inputs::ActiveModel {
        valuation_id: Set(valuation.id),
        average_dividend: Set(request.average_dividend),
        desired_yield: Set(request.desired_yield),
        ..Default::default()
    }
    .insert(db.inner())
    .await?;

    Ok(BazinValuationResponse { valuation, inputs })
}
