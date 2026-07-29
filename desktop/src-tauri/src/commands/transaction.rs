use std::collections::HashMap;

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};

use crate::entity::{assets, custodia, transactions};
use crate::error::AppError;

const BUY: &str = "compra";
const SELL: &str = "venda";
const DEPOSIT: &str = "aporte";
const WITHDRAWAL: &str = "retirada";
const DIVIDEND: &str = "provento";
const TRANSFER: &str = "transferencia";
const TRANSACTION_TYPES: [&str; 6] = [BUY, SELL, DEPOSIT, WITHDRAWAL, DIVIDEND, TRANSFER];

#[derive(Deserialize)]
pub struct CreateTransactionRequest {
    pub portfolio_id: i32,
    pub asset_id: Option<i32>,
    pub custodia_id: Option<i32>,
    pub transfer_to_custodia_id: Option<i32>,
    pub transaction_type: String,
    pub quantity: Option<f64>,
    pub unit_price: Option<f64>,
    pub total_value: f64,
    pub fee: Option<f64>,
    pub transaction_date: String,
    pub notes: Option<String>,
    pub fi_emissor: Option<String>,
    pub fi_indexador: Option<String>,
    pub fi_taxa_percentual: Option<f64>,
    pub fi_data_vencimento: Option<String>,
    pub fi_liquidez: Option<String>,
}

// Cross-field validation gating (não uma conta pura, mesmo espírito de
// `create_alert_rule`'s guard por `target_type`) — cada tipo de lançamento
// tem um conjunto diferente de campos obrigatórios (ver PHASE.md item 9 e
// a modelagem da Sessão 33 pra `transactions`).
fn validate(request: &CreateTransactionRequest) -> Result<(), AppError> {
    if !TRANSACTION_TYPES.contains(&request.transaction_type.as_str()) {
        return Err(AppError::InvalidGuard(format!(
            "transaction_type '{}' inválido (esperado um de {TRANSACTION_TYPES:?})",
            request.transaction_type
        )));
    }

    match request.transaction_type.as_str() {
        BUY | SELL => {
            if request.asset_id.is_none() {
                return Err(AppError::InvalidGuard(format!(
                    "asset_id é obrigatório para '{}'",
                    request.transaction_type
                )));
            }
            if request.quantity.is_none() || request.unit_price.is_none() {
                return Err(AppError::InvalidGuard(format!(
                    "quantity e unit_price são obrigatórios para '{}'",
                    request.transaction_type
                )));
            }
        }
        DEPOSIT | WITHDRAWAL => {
            if request.asset_id.is_some() {
                return Err(AppError::InvalidGuard(format!(
                    "'{}' é fluxo de caixa puro, não referencia um ativo",
                    request.transaction_type
                )));
            }
        }
        DIVIDEND => {
            if request.asset_id.is_none() {
                return Err(AppError::InvalidGuard(
                    "asset_id é obrigatório para 'provento' (qual ativo pagou)".to_string(),
                ));
            }
        }
        TRANSFER => {
            if request.asset_id.is_none() || request.quantity.is_none() {
                return Err(AppError::InvalidGuard(
                    "asset_id e quantity são obrigatórios para 'transferencia'".to_string(),
                ));
            }
            match (request.custodia_id, request.transfer_to_custodia_id) {
                (Some(from), Some(to)) if from == to => {
                    return Err(AppError::InvalidGuard(
                        "origem e destino da transferência não podem ser a mesma custódia"
                            .to_string(),
                    ));
                }
                (Some(_), Some(_)) => {}
                _ => {
                    return Err(AppError::InvalidGuard(
                        "transferência exige custodia_id (origem) e transfer_to_custodia_id (destino)"
                            .to_string(),
                    ));
                }
            }
        }
        _ => unreachable!("já validado pelo contains acima"),
    }

    Ok(())
}

#[tauri::command]
pub async fn create_transaction(
    db: tauri::State<'_, DatabaseConnection>,
    request: CreateTransactionRequest,
) -> Result<transactions::Model, AppError> {
    validate(&request)?;

    if let Some(asset_id) = request.asset_id {
        assets::Entity::find_by_id(asset_id)
            .one(db.inner())
            .await?
            .ok_or_else(|| AppError::NotFound(format!("asset {asset_id}")))?;
    }

    let tx = transactions::ActiveModel {
        portfolio_id: Set(request.portfolio_id),
        asset_id: Set(request.asset_id),
        custodia_id: Set(request.custodia_id),
        transfer_to_custodia_id: Set(request.transfer_to_custodia_id),
        transaction_type: Set(request.transaction_type),
        quantity: Set(request.quantity),
        unit_price: Set(request.unit_price),
        total_value: Set(request.total_value),
        fee: Set(request.fee),
        transaction_date: Set(request.transaction_date),
        notes: Set(request.notes),
        fi_emissor: Set(request.fi_emissor),
        fi_indexador: Set(request.fi_indexador),
        fi_taxa_percentual: Set(request.fi_taxa_percentual),
        fi_data_vencimento: Set(request.fi_data_vencimento),
        fi_liquidez: Set(request.fi_liquidez),
        created_at: Set(chrono::Utc::now().to_rfc3339()),
        ..Default::default()
    }
    .insert(db.inner())
    .await?;

    Ok(tx)
}

#[tauri::command]
pub async fn delete_transaction(
    db: tauri::State<'_, DatabaseConnection>,
    transaction_id: i32,
) -> Result<(), AppError> {
    transactions::Entity::delete_by_id(transaction_id)
        .exec(db.inner())
        .await?;

    Ok(())
}

fn custodia_label(custodias: &HashMap<i32, custodia::Model>, id: Option<i32>) -> Option<String> {
    id.and_then(|id| custodias.get(&id))
        .map(|c| format!("{} ({})", c.instituicao, c.titular))
}

#[derive(Serialize)]
pub struct TransactionView {
    pub id: i32,
    pub asset_id: Option<i32>,
    pub ticker: Option<String>,
    pub custodia_id: Option<i32>,
    pub custodia_label: Option<String>,
    pub transfer_to_custodia_id: Option<i32>,
    pub transfer_to_custodia_label: Option<String>,
    pub transaction_type: String,
    pub quantity: Option<f64>,
    pub unit_price: Option<f64>,
    pub total_value: f64,
    pub fee: Option<f64>,
    pub transaction_date: String,
    pub notes: Option<String>,
    pub fi_emissor: Option<String>,
    pub fi_indexador: Option<String>,
    pub fi_taxa_percentual: Option<f64>,
    pub fi_data_vencimento: Option<String>,
    pub fi_liquidez: Option<String>,
    pub created_at: String,
}

#[tauri::command]
pub async fn list_transactions(
    db: tauri::State<'_, DatabaseConnection>,
    portfolio_id: i32,
) -> Result<Vec<TransactionView>, AppError> {
    let txs = transactions::Entity::find()
        .filter(transactions::Column::PortfolioId.eq(portfolio_id))
        .order_by_desc(transactions::Column::TransactionDate)
        .all(db.inner())
        .await?;

    let asset_ids: Vec<i32> = txs.iter().filter_map(|t| t.asset_id).collect();
    let assets_map: HashMap<i32, assets::Model> = if asset_ids.is_empty() {
        HashMap::new()
    } else {
        assets::Entity::find()
            .filter(assets::Column::Id.is_in(asset_ids))
            .all(db.inner())
            .await?
            .into_iter()
            .map(|a| (a.id, a))
            .collect()
    };

    let custodia_ids: Vec<i32> = txs
        .iter()
        .filter_map(|t| t.custodia_id)
        .chain(txs.iter().filter_map(|t| t.transfer_to_custodia_id))
        .collect();
    let custodias_map: HashMap<i32, custodia::Model> = if custodia_ids.is_empty() {
        HashMap::new()
    } else {
        custodia::Entity::find()
            .filter(custodia::Column::Id.is_in(custodia_ids))
            .all(db.inner())
            .await?
            .into_iter()
            .map(|c| (c.id, c))
            .collect()
    };

    let views = txs
        .into_iter()
        .map(|tx| TransactionView {
            ticker: tx.asset_id.and_then(|id| assets_map.get(&id)).map(|a| a.ticker.clone()),
            custodia_label: custodia_label(&custodias_map, tx.custodia_id),
            transfer_to_custodia_label: custodia_label(&custodias_map, tx.transfer_to_custodia_id),
            id: tx.id,
            asset_id: tx.asset_id,
            custodia_id: tx.custodia_id,
            transfer_to_custodia_id: tx.transfer_to_custodia_id,
            transaction_type: tx.transaction_type,
            quantity: tx.quantity,
            unit_price: tx.unit_price,
            total_value: tx.total_value,
            fee: tx.fee,
            transaction_date: tx.transaction_date,
            notes: tx.notes,
            fi_emissor: tx.fi_emissor,
            fi_indexador: tx.fi_indexador,
            fi_taxa_percentual: tx.fi_taxa_percentual,
            fi_data_vencimento: tx.fi_data_vencimento,
            fi_liquidez: tx.fi_liquidez,
            created_at: tx.created_at,
        })
        .collect();

    Ok(views)
}

#[derive(Serialize)]
pub struct CustodiaBreakdown {
    pub custodia_id: Option<i32>,
    pub custodia_label: Option<String>,
    pub quantity: f64,
}

// MVP deliberadamente simples (decisão implícita da Sessão 33, escopo desta
// fatia): preço médio é a média das compras, sem redução proporcional do
// custo por venda (FIFO/custo médio "de verdade" fica pra quando a Fase
// 10.3 — rentabilidade histórica — for desenhada). `provento` não move
// quantidade (é caixa recebido, não ativo). `transferencia` move quantidade
// entre custódias sem alterar o total líquido do ativo no portfolio.
#[derive(Serialize)]
pub struct PositionView {
    pub asset_id: i32,
    pub ticker: String,
    pub name: String,
    pub asset_class: String,
    pub currency: String,
    pub quantity: f64,
    pub average_buy_price: Option<f64>,
    pub by_custodia: Vec<CustodiaBreakdown>,
}

#[tauri::command]
pub async fn get_portfolio_positions(
    db: tauri::State<'_, DatabaseConnection>,
    portfolio_id: i32,
) -> Result<Vec<PositionView>, AppError> {
    let txs = transactions::Entity::find()
        .filter(transactions::Column::PortfolioId.eq(portfolio_id))
        .all(db.inner())
        .await?;

    let mut by_asset: HashMap<i32, Vec<&transactions::Model>> = HashMap::new();
    for tx in &txs {
        if let Some(asset_id) = tx.asset_id {
            by_asset.entry(asset_id).or_default().push(tx);
        }
    }

    if by_asset.is_empty() {
        return Ok(vec![]);
    }

    let asset_ids: Vec<i32> = by_asset.keys().copied().collect();
    let assets_map: HashMap<i32, assets::Model> = assets::Entity::find()
        .filter(assets::Column::Id.is_in(asset_ids))
        .all(db.inner())
        .await?
        .into_iter()
        .map(|a| (a.id, a))
        .collect();

    let custodia_ids: Vec<i32> = txs
        .iter()
        .filter_map(|t| t.custodia_id)
        .chain(txs.iter().filter_map(|t| t.transfer_to_custodia_id))
        .collect();
    let custodias_map: HashMap<i32, custodia::Model> = if custodia_ids.is_empty() {
        HashMap::new()
    } else {
        custodia::Entity::find()
            .filter(custodia::Column::Id.is_in(custodia_ids))
            .all(db.inner())
            .await?
            .into_iter()
            .map(|c| (c.id, c))
            .collect()
    };

    let mut positions = Vec::new();
    for (asset_id, asset_txs) in by_asset {
        let Some(asset) = assets_map.get(&asset_id) else {
            continue;
        };

        let mut net_qty = 0.0;
        let mut buy_qty_sum = 0.0;
        let mut buy_value_sum = 0.0;
        let mut custodia_qty: HashMap<Option<i32>, f64> = HashMap::new();

        for tx in asset_txs {
            match tx.transaction_type.as_str() {
                BUY => {
                    let qty = tx.quantity.unwrap_or(0.0);
                    net_qty += qty;
                    buy_qty_sum += qty;
                    buy_value_sum += tx.total_value;
                    *custodia_qty.entry(tx.custodia_id).or_insert(0.0) += qty;
                }
                SELL => {
                    let qty = tx.quantity.unwrap_or(0.0);
                    net_qty -= qty;
                    *custodia_qty.entry(tx.custodia_id).or_insert(0.0) -= qty;
                }
                TRANSFER => {
                    let qty = tx.quantity.unwrap_or(0.0);
                    *custodia_qty.entry(tx.custodia_id).or_insert(0.0) -= qty;
                    *custodia_qty.entry(tx.transfer_to_custodia_id).or_insert(0.0) += qty;
                }
                _ => {}
            }
        }

        let average_buy_price = (buy_qty_sum > 0.0).then_some(buy_value_sum / buy_qty_sum);

        let by_custodia = custodia_qty
            .into_iter()
            .map(|(cid, quantity)| CustodiaBreakdown {
                custodia_id: cid,
                custodia_label: custodia_label(&custodias_map, cid),
                quantity,
            })
            .collect();

        positions.push(PositionView {
            asset_id,
            ticker: asset.ticker.clone(),
            name: asset.name.clone(),
            asset_class: asset.asset_class.clone(),
            currency: asset.currency.clone(),
            quantity: net_qty,
            average_buy_price,
            by_custodia,
        });
    }

    positions.sort_by(|a, b| a.ticker.cmp(&b.ticker));
    Ok(positions)
}
