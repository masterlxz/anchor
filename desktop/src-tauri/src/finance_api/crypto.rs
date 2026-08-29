// Fase 14.4 (fatia 2) — porta pra Rust os dois caminhos de cripto do
// coletor Python: `collect_crypto_ticker` (quote+histórico de preço de
// qualquer moeda, usado pelo Portfolio/Research) e `main_crypto`/
// `_record_crypto_indicator` (os indicadores do ciclo ETH, hardcoded
// `coin="ETH"`, usados só na tela de score cripto — sem relação com o
// primeiro). Eram 4 automatizados (Sessão 91); mais 4 se juntaram na
// automação depois que a Fase 1.12 do easybusiness achou fonte gratuita
// pra eles (CoinMetrics, ver `ETH_INDICATORS` abaixo) — só `staking_yield`
// (dos 9 originais da Fase 3) segue manual.
use chrono::Utc;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::domain::crypto_score::{self, Threshold};
use crate::entity::{crypto_fear_greed, crypto_indicators, indicator_thresholds, stock_price_history, stock_quotes};
use crate::error::AppError;
use crate::finance_api::{client, insert_ignoring_conflicts, FinanceApiHandle};

pub struct CryptoTickerResult {
    pub symbol: String,
    pub name: String,
    pub price: f64,
    pub history_points: usize,
    pub fear_greed_value: i32,
    pub fear_greed_classification: String,
}

/// Quote + histórico diário pra uma moeda qualquer (Fase 10, item 8 — cripto
/// virou classe de ativo do Portfolio). Uma chamada de `market_chart` do
/// lado da Finance API já dá os dois; Fear & Greed é global de mercado (não
/// por moeda), atualizado de graça junto — `ON CONFLICT DO NOTHING` no
/// índice único de `reading_date`, só grava a primeira vez no dia.
pub async fn collect_ticker(
    db: &DatabaseConnection,
    handle: &FinanceApiHandle,
    symbol: &str,
) -> Result<CryptoTickerResult, AppError> {
    let quote = client::fetch_crypto_quote(handle, symbol).await?;
    let history = client::fetch_crypto_price_history(handle, symbol).await?;
    if history.data.is_empty() {
        return Err(AppError::FinanceApi(format!(
            "Finance API returned no price history for '{symbol}' ({})",
            quote.coin_id
        )));
    }

    let now = Utc::now().to_rfc3339();
    let history_points = history.data.len();

    stock_quotes::ActiveModel {
        id: ActiveValue::NotSet,
        ticker: Set(symbol.to_string()),
        price: Set(quote.price),
        source: Set("coingecko".to_string()),
        fetched_at: Set(now.clone()),
        name: Set(Some(quote.name.clone())),
        exchange: Set(Some("CoinGecko".to_string())),
        currency: Set(Some("USD".to_string())),
    }
    .insert(db)
    .await?;

    let price_models: Vec<stock_price_history::ActiveModel> = history
        .data
        .into_iter()
        .map(|point| stock_price_history::ActiveModel {
            id: ActiveValue::NotSet,
            ticker: Set(symbol.to_string()),
            price_date: Set(point.price_date.to_string()),
            close_price: Set(point.price),
            source: Set("coingecko".to_string()),
            fetched_at: Set(now.clone()),
        })
        .collect();
    insert_ignoring_conflicts::<stock_price_history::Entity>(
        db,
        price_models,
        OnConflict::columns([
            stock_price_history::Column::Ticker,
            stock_price_history::Column::PriceDate,
        ])
        .do_nothing()
        .to_owned(),
    )
    .await?;

    let fear_greed = client::fetch_fear_greed(handle).await?;
    insert_ignoring_conflicts::<crypto_fear_greed::Entity>(
        db,
        vec![crypto_fear_greed::ActiveModel {
            id: ActiveValue::NotSet,
            value: Set(fear_greed.value),
            classification: Set(fear_greed.classification.clone()),
            reading_date: Set(fear_greed.reading_date.to_string()),
            source: Set("alternative.me".to_string()),
            fetched_at: Set(now),
        }],
        OnConflict::column(crypto_fear_greed::Column::ReadingDate)
            .do_nothing()
            .to_owned(),
    )
    .await?;

    Ok(CryptoTickerResult {
        symbol: symbol.to_string(),
        name: quote.name,
        price: quote.price,
        history_points,
        fear_greed_value: fear_greed.value,
        fear_greed_classification: fear_greed.classification,
    })
}

// Os 4 primeiros são automatizados desde a Fase 1.6 do easybusiness (Sessão
// 91 deste repo); os 4 últimos foram desbloqueados na Fase 1.12 dele
// (CoinMetrics Community API, sem chave, achado ao vivo nesta sessão) — o
// 5º indicador manual (`staking_yield`) segue sem fonte gratuita conhecida,
// fica de fora deste array de propósito.
const ETH_INDICATORS: [(&str, &str, &str); 8] = [
    // (nome interno / chave de indicator_thresholds, código na Finance API, source)
    ("tvl_trend", "tvl-trend", "defillama"),
    ("net_issuance", "net-issuance", "ultrasound.money"),
    ("fees_vs_emission", "fees-vs-emission", "ultrasound.money"),
    ("nvt_ratio", "nvt-ratio", "coingecko"),
    ("mvrv_z_score", "mvrv-z-score", "coinmetrics"),
    ("puell_multiple", "puell-multiple", "coinmetrics"),
    ("exchange_netflow", "exchange-netflow", "coinmetrics"),
    ("active_addresses_trend", "active-addresses-trend", "coinmetrics"),
];

async fn record_crypto_indicator(
    db: &DatabaseConnection,
    indicator: &str,
    raw_value: f64,
    source: &str,
) -> Result<crypto_indicators::Model, AppError> {
    let threshold = indicator_thresholds::Entity::find()
        .filter(indicator_thresholds::Column::Indicator.eq(indicator))
        .one(db)
        .await?
        .ok_or_else(|| AppError::UnknownIndicator(indicator.to_string()))?;

    let signal = crypto_score::classify(
        raw_value,
        &Threshold {
            green_boundary: threshold.green_boundary,
            red_boundary: threshold.red_boundary,
        },
    )?;

    let model = crypto_indicators::ActiveModel {
        id: ActiveValue::NotSet,
        coin: Set("ETH".to_string()),
        indicator: Set(indicator.to_string()),
        reading_date: Set(Utc::now().date_naive().to_string()),
        raw_value: Set(raw_value),
        signal: Set(signal.as_str().to_string()),
        source: Set(source.to_string()),
        created_at: Set(Utc::now().to_rfc3339()),
    }
    .insert(db)
    .await?;

    Ok(model)
}

/// Os 4 indicadores do ciclo ETH (`run_crypto_collector`) — sempre os
/// mesmos 4, sempre `coin = "ETH"`, mesma ordem de `main_crypto()`.
pub async fn collect_eth_indicators(
    db: &DatabaseConnection,
    handle: &FinanceApiHandle,
) -> Result<Vec<crypto_indicators::Model>, AppError> {
    let mut results = Vec::with_capacity(ETH_INDICATORS.len());
    for (internal_name, api_code, source) in ETH_INDICATORS {
        let raw_value = client::fetch_eth_indicator(handle, api_code).await?.raw_value;
        results.push(record_crypto_indicator(db, internal_name, raw_value, source).await?);
    }
    Ok(results)
}

// ---------------------------------------------------------------------------
// Testes `#[ignore]` — mesma convenção de `finance_api::stocks::tests`:
// precisam da Finance API real em `http://localhost:8000` e do banco real
// de dev. Rodar com `cargo test --lib -- --ignored finance_api::crypto`.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;

    async fn dev_db() -> DatabaseConnection {
        Database::connect("sqlite:///data/anchor.db?mode=rwc")
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
    async fn live_collect_ticker_writes_real_rows() {
        let db = dev_db().await;
        let handle = handle();

        let result = collect_ticker(&db, &handle, "BTC").await.unwrap();
        assert!(result.price > 0.0);
        assert!(result.history_points > 0);
    }

    #[tokio::test]
    #[ignore]
    async fn live_collect_eth_indicators_writes_eight_readings() {
        let db = dev_db().await;
        let handle = handle();

        let readings = collect_eth_indicators(&db, &handle).await.unwrap();
        assert_eq!(readings.len(), 8);
        for reading in &readings {
            assert!(matches!(reading.signal.as_str(), "GREEN" | "NEUTRAL" | "RED"));
        }
    }
}
