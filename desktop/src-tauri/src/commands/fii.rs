use std::sync::atomic::{AtomicBool, Ordering};

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

use crate::commands::collector::CollectorSummary;
use crate::entity::{fii_cvm_monthly, fii_cvm_properties};
use crate::error::AppError;
use crate::finance_api::{fii as finance_api_fii, FinanceApiHandle};

/// Fase 10, item 8, Sessão 41 — sugestão de CNPJ pro cadastro de um FII,
/// resolvida cruzando bolsai + cadastro público da CVM (nunca chuta — 0 ou
/// mais de um match vira `None`, ver docstring de
/// `cvm_fii.py::resolve_cnpj`/`app.sources.cvm_fii.resolve_cnpj` do
/// easybusiness). O usuário confirma/edita antes de salvar em `assets.cnpj`
/// — este comando não escreve nada lá, só no cache de resolução.
#[derive(Serialize, Deserialize)]
pub struct FiiCnpjSuggestion {
    pub cnpj: String,
    pub fund_name: String,
}

/// Fase 14.4 (Sessão 92) — trocou de `run_collector` (subprocess Python,
/// `--fii-resolve-cnpj`) pra `finance_api::fii::resolve_cnpj` direto,
/// desbloqueado pela Fase 1.11.3 do easybusiness. Sem `lock`/`app` — a
/// chamada HTTP não precisa do lock de subprocess (frontend só passa
/// `{ ticker }`, ver `AssetSection.tsx`/`FiiLookupSection.tsx`).
#[tauri::command]
pub async fn resolve_fii_cnpj(
    db: tauri::State<'_, DatabaseConnection>,
    finance_api: tauri::State<'_, FinanceApiHandle>,
    ticker: String,
) -> Result<Option<FiiCnpjSuggestion>, AppError> {
    let resolution = finance_api_fii::resolve_cnpj(db.inner(), &finance_api, &ticker).await?;

    Ok(resolution.map(|r| FiiCnpjSuggestion {
        cnpj: r.cnpj,
        fund_name: r.fund_name,
    }))
}

/// Puxa indicadores mensais + imóveis (vacância/inadimplência) direto da
/// CVM pra uma lista de CNPJs já resolvidos e salvos — a bolsai não entra
/// aqui, só na sugestão inicial (`resolve_fii_cnpj` acima).
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
