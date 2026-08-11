use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, Unchanged,
};
use serde::{Deserialize, Serialize};

use crate::entity::{general_transaction, general_transaction_category};
use crate::error::AppError;

#[derive(Deserialize)]
pub struct CreateGeneralTransactionCategoryRequest {
    pub workspace_id: i32,
    pub nome: String,
    pub limite_mensal: Option<f64>,
}

#[tauri::command]
pub async fn create_general_transaction_category(
    db: tauri::State<'_, DatabaseConnection>,
    request: CreateGeneralTransactionCategoryRequest,
) -> Result<general_transaction_category::Model, AppError> {
    let category = general_transaction_category::ActiveModel {
        workspace_id: Set(request.workspace_id),
        nome: Set(request.nome),
        limite_mensal: Set(request.limite_mensal),
        created_at: Set(chrono::Utc::now().to_rfc3339()),
        ..Default::default()
    }
    .insert(db.inner())
    .await?;

    Ok(category)
}

#[derive(Deserialize)]
pub struct UpdateGeneralTransactionCategoryRequest {
    pub general_transaction_category_id: i32,
    pub nome: String,
    pub limite_mensal: Option<f64>,
}

#[tauri::command]
pub async fn update_general_transaction_category(
    db: tauri::State<'_, DatabaseConnection>,
    request: UpdateGeneralTransactionCategoryRequest,
) -> Result<general_transaction_category::Model, AppError> {
    let existing =
        general_transaction_category::Entity::find_by_id(request.general_transaction_category_id)
            .one(db.inner())
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "general transaction category {}",
                    request.general_transaction_category_id
                ))
            })?;

    let updated = general_transaction_category::ActiveModel {
        id: Unchanged(existing.id),
        workspace_id: Unchanged(existing.workspace_id),
        nome: Set(request.nome),
        limite_mensal: Set(request.limite_mensal),
        ..Default::default()
    }
    .update(db.inner())
    .await?;

    Ok(updated)
}

#[tauri::command]
pub async fn delete_general_transaction_category(
    db: tauri::State<'_, DatabaseConnection>,
    general_transaction_category_id: i32,
) -> Result<(), AppError> {
    general_transaction_category::Entity::delete_by_id(general_transaction_category_id)
        .exec(db.inner())
        .await?;

    Ok(())
}

#[derive(Serialize)]
pub struct GeneralTransactionCategoryView {
    pub id: i32,
    pub workspace_id: i32,
    pub nome: String,
    pub limite_mensal: Option<f64>,
    pub created_at: String,
    pub spent_this_month: f64,
}

/// Soma o `valor` das transações `despesa` daquela categoria cujo
/// `transaction_date` cai no mês `month_prefix` (`"YYYY-MM"`) — limite de
/// orçamento é sobre gasto, não sobre `receita`/`parcela_divida`. Função
/// pura (sem I/O), mesmo espírito de `compute_balances`
/// (`commands/bank_account.rs`).
fn compute_spent_this_month(
    transactions: &[general_transaction::Model],
    category_id: i32,
    month_prefix: &str,
) -> f64 {
    transactions
        .iter()
        .filter(|tx| {
            tx.transaction_type == "despesa"
                && tx.category_id == Some(category_id)
                && tx.transaction_date.starts_with(month_prefix)
        })
        .map(|tx| tx.valor)
        .sum()
}

#[tauri::command]
pub async fn list_general_transaction_categories(
    db: tauri::State<'_, DatabaseConnection>,
    workspace_id: i32,
) -> Result<Vec<GeneralTransactionCategoryView>, AppError> {
    let categories = general_transaction_category::Entity::find()
        .filter(general_transaction_category::Column::WorkspaceId.eq(workspace_id))
        .all(db.inner())
        .await?;

    let transactions = general_transaction::Entity::find()
        .filter(general_transaction::Column::WorkspaceId.eq(workspace_id))
        .all(db.inner())
        .await?;

    let month_prefix = chrono::Utc::now().format("%Y-%m").to_string();

    Ok(categories
        .into_iter()
        .map(|category| GeneralTransactionCategoryView {
            spent_this_month: compute_spent_this_month(&transactions, category.id, &month_prefix),
            id: category.id,
            workspace_id: category.workspace_id,
            nome: category.nome,
            limite_mensal: category.limite_mensal,
            created_at: category.created_at,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tx(
        category_id: Option<i32>,
        transaction_type: &str,
        valor: f64,
        date: &str,
    ) -> general_transaction::Model {
        general_transaction::Model {
            id: 0,
            workspace_id: 1,
            bank_account_id: 1,
            transaction_type: transaction_type.to_string(),
            category_id,
            valor,
            transaction_date: date.to_string(),
            notes: None,
            created_at: "2026-08-11T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn compute_spent_this_month_is_zero_for_no_transactions() {
        assert_eq!(compute_spent_this_month(&[], 1, "2026-08"), 0.0);
    }

    #[test]
    fn compute_spent_this_month_sums_despesa_for_category_and_month() {
        let transactions = vec![
            tx(Some(1), "despesa", 100.0, "2026-08-05"),
            tx(Some(1), "despesa", 50.0, "2026-08-20"),
        ];
        assert_eq!(compute_spent_this_month(&transactions, 1, "2026-08"), 150.0);
    }

    #[test]
    fn compute_spent_this_month_ignores_receita() {
        let transactions = vec![tx(Some(1), "receita", 100.0, "2026-08-05")];
        assert_eq!(compute_spent_this_month(&transactions, 1, "2026-08"), 0.0);
    }

    #[test]
    fn compute_spent_this_month_ignores_other_category() {
        let transactions = vec![tx(Some(2), "despesa", 100.0, "2026-08-05")];
        assert_eq!(compute_spent_this_month(&transactions, 1, "2026-08"), 0.0);
    }

    #[test]
    fn compute_spent_this_month_ignores_other_month() {
        let transactions = vec![tx(Some(1), "despesa", 100.0, "2026-07-31")];
        assert_eq!(compute_spent_this_month(&transactions, 1, "2026-08"), 0.0);
    }
}
