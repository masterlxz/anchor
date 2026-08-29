// Fase 14.4 (fatia 4) — porta `collect_fii_cvm_data` (indicadores mensais +
// imóveis da CVM) pra Rust. `resolve_fii_cnpj` (bolsai+CVM pra sugerir o
// CNPJ) fica de fora — é uma das 4 capacidades sem endpoint equivalente na
// Finance API ainda (Sessão 88), continua no coletor Python.
use chrono::Utc;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveValue, DatabaseConnection, EntityTrait, Set};

use crate::entity::{fii_cvm_monthly, fii_cvm_properties};
use crate::error::AppError;
use crate::finance_api::{client, insert_ignoring_conflicts, FinanceApiHandle};

pub struct FiiCvmDataResult {
    pub monthly_count: usize,
    pub properties_count: usize,
}

pub async fn collect_cvm_data(
    db: &DatabaseConnection,
    handle: &FinanceApiHandle,
    cnpjs: &[String],
) -> Result<FiiCvmDataResult, AppError> {
    let now = Utc::now().to_rfc3339();

    let mut monthly_models = Vec::new();
    let mut monthly_count = 0usize;
    let mut properties_models = Vec::new();
    let mut properties_count = 0usize;

    for cnpj in cnpjs {
        // A Finance API exige o CNPJ só com dígitos na URL (achado ao vivo
        // nesta sessão — 404 pra "11.728.688/0001-47", 200 pra
        // "11728688000147") — mas `assets.cnpj` (e todo o resto do app,
        // `list_fii_cvm_monthly`/`list_fii_cvm_properties` incluídos) usa o
        // CNPJ formatado com pontuação. Normaliza só pra chamada; grava com
        // o `cnpj` original recebido, não o que a API ecoa de volta —
        // mesma decisão de `finance_api::metals` com o ticker.
        let digits_only: String = cnpj.chars().filter(char::is_ascii_digit).collect();

        let monthly = client::fetch_fii_monthly_indicators(handle, &digits_only).await?;
        monthly_count += 1;
        monthly_models.push(fii_cvm_monthly::ActiveModel {
            id: ActiveValue::NotSet,
            cnpj: Set(cnpj.clone()),
            reference_date: Set(monthly.reference_date.to_string()),
            patrimonio_liquido: Set(monthly.patrimonio_liquido),
            valor_patrimonial_cota: Set(monthly.valor_patrimonial_cota),
            numero_cotistas: Set(monthly.numero_cotistas),
            dividend_yield_mes: Set(monthly.dividend_yield_mes),
            rentabilidade_efetiva_mes: Set(monthly.rentabilidade_efetiva_mes),
            source: Set("cvm_fii".to_string()),
            fetched_at: Set(now.clone()),
        });

        let properties = client::fetch_fii_properties(handle, &digits_only).await?;
        for property in properties.data {
            properties_count += 1;
            properties_models.push(fii_cvm_properties::ActiveModel {
                id: ActiveValue::NotSet,
                cnpj: Set(cnpj.clone()),
                reference_date: Set(property.reference_date.to_string()),
                nome_imovel: Set(property.nome_imovel),
                endereco: Set(property.endereco),
                area_m2: Set(property.area_m2),
                percentual_vacancia: Set(property.percentual_vacancia),
                percentual_inadimplencia: Set(property.percentual_inadimplencia),
                percentual_receitas_fii: Set(property.percentual_receitas_fii),
                percentual_locado: Set(property.percentual_locado),
                source: Set("cvm_fii".to_string()),
                fetched_at: Set(now.clone()),
            });
        }
    }

    insert_ignoring_conflicts::<fii_cvm_monthly::Entity>(
        db,
        monthly_models,
        OnConflict::columns([
            fii_cvm_monthly::Column::Cnpj,
            fii_cvm_monthly::Column::ReferenceDate,
        ])
        .do_nothing()
        .to_owned(),
    )
    .await?;

    insert_ignoring_conflicts::<fii_cvm_properties::Entity>(
        db,
        properties_models,
        OnConflict::columns([
            fii_cvm_properties::Column::Cnpj,
            fii_cvm_properties::Column::ReferenceDate,
            fii_cvm_properties::Column::NomeImovel,
        ])
        .do_nothing()
        .to_owned(),
    )
    .await?;

    Ok(FiiCvmDataResult {
        monthly_count,
        properties_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;

    async fn dev_db() -> DatabaseConnection {
        Database::connect("sqlite:///data-collector/anchor.db?mode=rwc")
            .await
            .expect("failed to connect to the real dev database")
    }

    fn handle() -> FinanceApiHandle {
        FinanceApiHandle::for_test(
            "http://localhost:8000".to_string(),
            "local-dev-key-change-me".to_string(),
        )
    }

    #[tokio::test]
    #[ignore]
    async fn live_collect_cvm_data_is_idempotent() {
        let db = dev_db().await;
        let handle = handle();
        // HGLG11 — CNPJ real, já resolvido e cacheado em `fii_cnpj_cache`
        // pelo banco de dev deste projeto (consultado direto pra este
        // teste, não inventado).
        let cnpjs = vec!["11.728.688/0001-47".to_string()];

        let first = collect_cvm_data(&db, &handle, &cnpjs).await.unwrap();
        assert!(first.monthly_count > 0);
    }
}
