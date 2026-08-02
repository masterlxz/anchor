use std::sync::atomic::{AtomicBool, Ordering};

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use serde::Serialize;
use tokio::process::Command;

use crate::entity::{
    crypto_fear_greed, stock_dcf_fundamentals, stock_dividend_payments, stock_dividends_avg,
    stock_fundamentals, stock_price_history, stock_quotes, stock_technicals,
};
use crate::error::AppError;

// Dev-only paths — the venv and script live in the data-collector/ bind
// mount (see docker-compose.yml), same absolute-path convention as
// db.rs's DATABASE_URL. Production path deferred to Fase 6.
const COLLECTOR_PYTHON: &str = "/data-collector/.venv/bin/python3";
const COLLECTOR_SCRIPT: &str = "/data-collector/main.py";

#[derive(Serialize)]
pub struct CollectorSummary {
    pub success: bool,
    pub output: String,
}

// `pub(crate)` — reaproveitado por `commands::fii` (Sessão 41), que também
// invoca o mesmo subprocess do coletor pra resolver CNPJ e puxar indicadores
// da CVM, mesmo lock/binário/script.
pub(crate) async fn run_collector(
    lock: &AtomicBool,
    extra_args: &[&str],
) -> Result<CollectorSummary, AppError> {
    if lock.swap(true, Ordering::SeqCst) {
        return Err(AppError::CollectorBusy);
    }

    let result = Command::new(COLLECTOR_PYTHON)
        .arg(COLLECTOR_SCRIPT)
        .args(extra_args)
        .output()
        .await;

    lock.store(false, Ordering::SeqCst);

    let output = result.map_err(|err| AppError::CollectorFailed(err.to_string()))?;

    let summary = if output.status.success() {
        CollectorSummary {
            success: true,
            output: String::from_utf8_lossy(&output.stdout).to_string(),
        }
    } else {
        CollectorSummary {
            success: false,
            output: String::from_utf8_lossy(&output.stderr).to_string(),
        }
    };

    Ok(summary)
}

// `asset_class` só distingue "cripto" (fonte CoinGecko, `--crypto-ticker`)
// do resto (Ação BR/FII/ETF, todos o mesmo endpoint Yahoo `.SA`, `--ticker`)
// — ver PHASE.md item 8, Sessão 51. `None` preserva o comportamento anterior
// (chamadores que não sabem/não precisam distinguir classe, ex.: os
// formulários de valuation via `useTickerCollector`).
#[tauri::command]
pub async fn run_stock_collector(
    lock: tauri::State<'_, AtomicBool>,
    ticker: String,
    asset_class: Option<String>,
) -> Result<CollectorSummary, AppError> {
    if asset_class.as_deref() == Some("cripto") {
        run_collector(&lock, &["--crypto-ticker", &ticker]).await
    } else {
        run_collector(&lock, &["--ticker", &ticker]).await
    }
}

#[tauri::command]
pub async fn run_crypto_collector(
    lock: tauri::State<'_, AtomicBool>,
) -> Result<CollectorSummary, AppError> {
    run_collector(&lock, &["crypto"]).await
}

#[tauri::command]
pub async fn list_stock_quotes(
    db: tauri::State<'_, DatabaseConnection>,
) -> Result<Vec<stock_quotes::Model>, AppError> {
    let quotes = stock_quotes::Entity::find()
        .order_by_desc(stock_quotes::Column::FetchedAt)
        .all(db.inner())
        .await?;

    Ok(quotes)
}

#[tauri::command]
pub async fn list_stock_fundamentals(
    db: tauri::State<'_, DatabaseConnection>,
) -> Result<Vec<stock_fundamentals::Model>, AppError> {
    let fundamentals = stock_fundamentals::Entity::find()
        .order_by_desc(stock_fundamentals::Column::FetchedAt)
        .all(db.inner())
        .await?;

    Ok(fundamentals)
}

#[tauri::command]
pub async fn list_stock_dividends_avg(
    db: tauri::State<'_, DatabaseConnection>,
) -> Result<Vec<stock_dividends_avg::Model>, AppError> {
    let dividends = stock_dividends_avg::Entity::find()
        .order_by_desc(stock_dividends_avg::Column::FetchedAt)
        .all(db.inner())
        .await?;

    Ok(dividends)
}

#[tauri::command]
pub async fn list_stock_dcf_fundamentals(
    db: tauri::State<'_, DatabaseConnection>,
) -> Result<Vec<stock_dcf_fundamentals::Model>, AppError> {
    let fundamentals = stock_dcf_fundamentals::Entity::find()
        .order_by_desc(stock_dcf_fundamentals::Column::FetchedAt)
        .all(db.inner())
        .await?;

    Ok(fundamentals)
}

#[tauri::command]
pub async fn list_stock_technicals(
    db: tauri::State<'_, DatabaseConnection>,
) -> Result<Vec<stock_technicals::Model>, AppError> {
    let technicals = stock_technicals::Entity::find()
        .order_by_desc(stock_technicals::Column::FetchedAt)
        .all(db.inner())
        .await?;

    Ok(technicals)
}

#[tauri::command]
pub async fn list_stock_dividend_payments(
    db: tauri::State<'_, DatabaseConnection>,
) -> Result<Vec<stock_dividend_payments::Model>, AppError> {
    let payments = stock_dividend_payments::Entity::find()
        .order_by_asc(stock_dividend_payments::Column::PaymentDate)
        .all(db.inner())
        .await?;

    Ok(payments)
}

#[tauri::command]
pub async fn run_price_history_backfill(
    lock: tauri::State<'_, AtomicBool>,
    tickers: Vec<String>,
) -> Result<CollectorSummary, AppError> {
    let joined = tickers.join(",");
    run_collector(&lock, &["--price-history", &joined]).await
}

#[tauri::command]
pub async fn list_stock_price_history(
    db: tauri::State<'_, DatabaseConnection>,
    ticker: String,
) -> Result<Vec<stock_price_history::Model>, AppError> {
    let history = stock_price_history::Entity::find()
        .filter(stock_price_history::Column::Ticker.eq(ticker))
        .order_by_asc(stock_price_history::Column::PriceDate)
        .all(db.inner())
        .await?;

    Ok(history)
}

// Fase 10, item 8, Sessão 51 — pedido explícito do dono do projeto: Fear &
// Greed Index em "toda tela que for cripto". Global (não por coin), então
// só a leitura mais recente interessa — `run_stock_collector` com
// `asset_class: "cripto"` já mantém `crypto_fear_greed` em dia (ver
// `collect_crypto_ticker` no coletor Python), esta é só a leitura.
#[tauri::command]
pub async fn get_latest_crypto_fear_greed(
    db: tauri::State<'_, DatabaseConnection>,
) -> Result<Option<crypto_fear_greed::Model>, AppError> {
    let latest = crypto_fear_greed::Entity::find()
        .order_by_desc(crypto_fear_greed::Column::ReadingDate)
        .one(db.inner())
        .await?;

    Ok(latest)
}
