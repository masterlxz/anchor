use std::sync::atomic::{AtomicBool, Ordering};

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

use crate::commands::collector::{run_collector, CollectorSummary};
use crate::entity::{fii_cvm_monthly, fii_cvm_properties};
use crate::error::AppError;
use crate::finance_api::{fii as finance_api_fii, FinanceApiHandle};

/// Fase 10, item 8, Sessão 41 — sugestão de CNPJ pro cadastro de um FII,
/// resolvida via `cvm_fii.py::resolve_cnpj` (bolsai só pra identificar o
/// fundo, cruzado contra o cadastro público da CVM — nunca chuta, ver
/// docstring do módulo Python). O usuário confirma/edita antes de salvar
/// em `assets.cnpj` — este comando não escreve nada no banco.
#[derive(Serialize, Deserialize)]
pub struct FiiCnpjSuggestion {
    pub cnpj: String,
    pub fund_name: String,
}

#[tauri::command]
pub async fn resolve_fii_cnpj(
    app: tauri::AppHandle,
    lock: tauri::State<'_, AtomicBool>,
    ticker: String,
) -> Result<Option<FiiCnpjSuggestion>, AppError> {
    let summary = run_collector(&app, &lock, &["--fii-resolve-cnpj", &ticker]).await?;
    if !summary.success {
        return Err(AppError::CollectorFailed(summary.output));
    }

    serde_json::from_str(summary.output.trim())
        .map_err(|err| AppError::CollectorFailed(format!("invalid collector output: {err}")))
}

/// Puxa indicadores mensais + imóveis (vacância/inadimplência) direto da
/// CVM pra uma lista de CNPJs já resolvidos e salvos — a bolsai não entra
/// aqui, só na sugestão inicial (`resolve_fii_cnpj`, que continua no
/// coletor Python — sem endpoint equivalente na Finance API ainda).
///
/// Fase 14.4 — trocou de `run_collector` (subprocess Python,
/// `--fii-cvm-data`) pra `finance_api::fii::collect_cvm_data` direto.
#[tauri::command]
pub async fn run_fii_cvm_collector(
    lock: tauri::State<'_, AtomicBool>,
    db: tauri::State<'_, DatabaseConnection>,
    finance_api: tauri::State<'_, FinanceApiHandle>,
    cnpjs: Vec<String>,
) -> Result<CollectorSummary, AppError> {
    if lock.swap(true, Ordering::SeqCst) {
        return Err(AppError::CollectorBusy);
    }
    let result = finance_api_fii::collect_cvm_data(db.inner(), &finance_api, &cnpjs)
        .await
        .map(|r| CollectorSummary {
            success: true,
            output: format!(
                "Fetched {} monthly indicator(s) and {} propert(y/ies)",
                r.monthly_count, r.properties_count,
            ),
        });
    lock.store(false, Ordering::SeqCst);

    result
}

#[tauri::command]
pub async fn list_fii_cvm_monthly(
    db: tauri::State<'_, DatabaseConnection>,
    cnpj: String,
) -> Result<Vec<fii_cvm_monthly::Model>, AppError> {
    let rows = fii_cvm_monthly::Entity::find()
        .filter(fii_cvm_monthly::Column::Cnpj.eq(cnpj))
        .order_by_asc(fii_cvm_monthly::Column::ReferenceDate)
        .all(db.inner())
        .await?;

    Ok(rows)
}

#[tauri::command]
pub async fn list_fii_cvm_properties(
    db: tauri::State<'_, DatabaseConnection>,
    cnpj: String,
) -> Result<Vec<fii_cvm_properties::Model>, AppError> {
    let rows = fii_cvm_properties::Entity::find()
        .filter(fii_cvm_properties::Column::Cnpj.eq(cnpj))
        .order_by_desc(fii_cvm_properties::Column::ReferenceDate)
        .all(db.inner())
        .await?;

    Ok(rows)
}
