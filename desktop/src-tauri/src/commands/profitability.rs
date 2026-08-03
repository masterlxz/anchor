use std::collections::{HashMap, HashSet};

use chrono::{Datelike, Duration, NaiveDate, Utc};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::Serialize;

use crate::domain::twr::{self, CashFlow};
use crate::entity::{assets, stock_price_history, transactions};
use crate::error::AppError;

const BUY: &str = "compra";
const SELL: &str = "venda";
const DIVIDEND: &str = "provento";

// Fase 10.3, fatia escopada às classes com preço histórico automatizado
// (decisão explícita do dono do projeto — Stocks internacionais/Tesouro
// Direto/Renda Fixa ainda não têm, ver project/PHASE.md). `aporte`/
// `retirada` (sem asset_id) ficam fora deste cálculo. `fii` entrou na
// Sessão 41, `etf_br` na Sessão 49 (2026-08-02) — mesmo endpoint Yahoo
// `{ticker}.SA` que `acao_br` já usa pra `stock_price_history`, sem
// coletor novo. `cripto` entrou na Sessão 51 — fonte diferente
// (CoinGecko), mas grava na mesma tabela genérica `stock_price_history`
// (`ticker`/`price_date`/`close_price`, sem coluna de fonte usada aqui),
// então esta consulta não distingue de onde o preço veio. `bdr` entrou na
// Sessão 53 — mesmo endpoint Yahoo `.SA` de `acao_br`/`fii`/`etf_br`, zero
// mudança de coletor. `metal` entrou na Sessão 55 — fonte Yahoo sem `.SA`
// (COMEX), mesma tabela genérica, mesmo raciocínio. Essa sub-carteira
// é tratada como autocontida: suas próprias compra(+)/venda(−) são os
// fluxos de caixa externos do Dietz Modificado — o app não modela um
// sub-ledger de caixa, então uma compra não é "caixa→ativo", é "ativo
// aparece", e pro efeito desta fatia isso equivale a um aporte na
// sub-carteira.
const ASSET_CLASSES_WITH_AUTO_QUOTE: [&str; 7] = [
    "acao_br",
    "fii",
    "etf_br",
    "cripto",
    "bdr",
    "metal",
    "acao_internacional",
];
const PRICE_TOLERANCE_DAYS: i64 = 7;

#[derive(Serialize)]
pub struct MonthlyReturn {
    pub year_month: String,
    pub bmv: f64,
    pub emv: f64,
    pub cf_total: f64,
    pub r_month_pct: f64,
    pub r_cumulative_pct: f64,
}

fn days_in_month(year: i32, month: u32) -> i64 {
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .expect("valid date");
    let start = NaiveDate::from_ymd_opt(year, month, 1).expect("valid date");
    (next - start).num_days()
}

// Peso de um fluxo dentro do mês: dia 1 ≈ quase todo o mês "trabalhado",
// último dia ≈ 0. Bate com o exemplo do PHASE.md (dia 10 de um mês de 31
// dias → 21/31 ≈ 0,6774).
fn flow_weight(date: NaiveDate) -> f64 {
    let days = days_in_month(date.year(), date.month());
    (days - date.day() as i64) as f64 / days as f64
}

// Fim do mês pro qual valorar EMV — pro mês corrente (ainda em curso), usa
// "hoje" em vez do último dia calendário (que ainda não aconteceu); ao
// rodar de novo já no mês seguinte, esse mesmo mês passa a usar seu último
// dia real.
fn month_end_date(year: i32, month: u32, today: NaiveDate) -> NaiveDate {
    let last_day = days_in_month(year, month);
    let candidate = NaiveDate::from_ymd_opt(year, month, last_day as u32).expect("valid date");
    candidate.min(today)
}

async fn closest_price(
    db: &DatabaseConnection,
    ticker: &str,
    target_date: NaiveDate,
) -> Result<Option<f64>, AppError> {
    let lower = (target_date - Duration::days(PRICE_TOLERANCE_DAYS))
        .format("%Y-%m-%d")
        .to_string();
    let upper = (target_date + Duration::days(PRICE_TOLERANCE_DAYS))
        .format("%Y-%m-%d")
        .to_string();

    let candidates = stock_price_history::Entity::find()
        .filter(stock_price_history::Column::Ticker.eq(ticker))
        .filter(stock_price_history::Column::PriceDate.gte(lower))
        .filter(stock_price_history::Column::PriceDate.lte(upper))
        .all(db)
        .await?;

    Ok(candidates
        .into_iter()
        .filter_map(|row| {
            NaiveDate::parse_from_str(&row.price_date, "%Y-%m-%d")
                .ok()
                .map(|d| (d, row.close_price))
        })
        .min_by_key(|(d, _)| (*d - target_date).num_days().abs())
        .map(|(_, price)| price))
}

#[tauri::command]
pub async fn get_portfolio_profitability(
    db: tauri::State<'_, DatabaseConnection>,
    portfolio_id: i32,
) -> Result<Vec<MonthlyReturn>, AppError> {
    let db = db.inner();

    let txs = transactions::Entity::find()
        .filter(transactions::Column::PortfolioId.eq(portfolio_id))
        .all(db)
        .await?;

    let asset_ids: Vec<i32> = txs.iter().filter_map(|t| t.asset_id).collect();
    let assets_map: HashMap<i32, assets::Model> = if asset_ids.is_empty() {
        HashMap::new()
    } else {
        assets::Entity::find()
            .filter(assets::Column::Id.is_in(asset_ids))
            .all(db)
            .await?
            .into_iter()
            .map(|a| (a.id, a))
            .collect()
    };

    let auto_quote_ids: HashSet<i32> = assets_map
        .values()
        .filter(|a| ASSET_CLASSES_WITH_AUTO_QUOTE.contains(&a.asset_class.as_str()))
        .map(|a| a.id)
        .collect();

    let mut relevant: Vec<&transactions::Model> = txs
        .iter()
        .filter(|t| {
            matches!(t.transaction_type.as_str(), BUY | SELL | DIVIDEND)
                && t.asset_id.is_some_and(|id| auto_quote_ids.contains(&id))
        })
        .collect();
    relevant.sort_by(|a, b| a.transaction_date.cmp(&b.transaction_date));

    if relevant.is_empty() {
        return Ok(vec![]);
    }

    let today = Utc::now().date_naive();
    let first_date = NaiveDate::parse_from_str(&relevant[0].transaction_date, "%Y-%m-%d")
        .map_err(|_| AppError::InvalidGuard("data de transação inválida".to_string()))?;

    let mut months: Vec<(i32, u32)> = Vec::new();
    let (mut y, mut m) = (first_date.year(), first_date.month());
    while (y, m) <= (today.year(), today.month()) {
        months.push((y, m));
        m += 1;
        if m == 13 {
            m = 1;
            y += 1;
        }
    }

    let mut holdings: HashMap<i32, f64> = HashMap::new();
    let mut bmv = 0.0;
    let mut monthly_r: Vec<f64> = Vec::new();
    let mut results = Vec::new();

    for (year, month) in months {
        let month_prefix = format!("{year:04}-{month:02}");
        let month_txs: Vec<&&transactions::Model> = relevant
            .iter()
            .filter(|t| t.transaction_date.starts_with(&month_prefix))
            .collect();

        let mut cash_flows = Vec::new();
        for tx in &month_txs {
            let date = NaiveDate::parse_from_str(&tx.transaction_date, "%Y-%m-%d")
                .map_err(|_| AppError::InvalidGuard("data de transação inválida".to_string()))?;
            let weight = flow_weight(date);
            match tx.transaction_type.as_str() {
                BUY => {
                    let qty = tx.quantity.unwrap_or(0.0);
                    *holdings.entry(tx.asset_id.unwrap()).or_insert(0.0) += qty;
                    cash_flows.push(CashFlow {
                        amount: tx.total_value,
                        weight,
                    });
                }
                SELL => {
                    let qty = tx.quantity.unwrap_or(0.0);
                    *holdings.entry(tx.asset_id.unwrap()).or_insert(0.0) -= qty;
                    cash_flows.push(CashFlow {
                        amount: -tx.total_value,
                        weight,
                    });
                }
                DIVIDEND => {
                    cash_flows.push(CashFlow {
                        amount: tx.total_value,
                        weight,
                    });
                }
                _ => {}
            }
        }

        let target_date = month_end_date(year, month, today);
        let mut emv = 0.0;
        for (&asset_id, &qty) in holdings.iter() {
            if qty.abs() < 1e-9 {
                continue;
            }
            let ticker = &assets_map[&asset_id].ticker;
            let price = closest_price(db, ticker, target_date).await?.ok_or_else(|| {
                AppError::NotFound(format!(
                    "sem preço histórico de {ticker} perto de {target_date} — rode \"Atualizar histórico de preços\""
                ))
            })?;
            emv += qty * price;
        }

        let r_month = if bmv == 0.0 && emv == 0.0 && cash_flows.is_empty() {
            0.0
        } else {
            twr::modified_dietz(bmv, emv, &cash_flows)?
        };

        monthly_r.push(r_month);
        let cf_total: f64 = cash_flows.iter().map(|cf| cf.amount).sum();

        results.push(MonthlyReturn {
            year_month: month_prefix,
            bmv,
            emv,
            cf_total,
            r_month_pct: r_month * 100.0,
            r_cumulative_pct: twr::chain(&monthly_r) * 100.0,
        });

        bmv = emv;
    }

    Ok(results)
}
