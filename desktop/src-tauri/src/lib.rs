use std::sync::atomic::AtomicBool;

mod alert_checker;
mod commands;
mod db;
mod dead_drop;
mod domain;
mod ecies;
mod entity;
mod error;
mod ipns_key;
mod lan_sweep;
mod sync_registry;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db = tauri::async_runtime::block_on(db::connect()).expect("failed to connect to database");
    alert_checker::spawn_periodic_check(db.clone());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(db)
        .manage(AtomicBool::new(false))
        .invoke_handler(tauri::generate_handler![
            commands::bazin::calculate_bazin,
            commands::graham::calculate_graham,
            commands::gordon::calculate_gordon,
            commands::dcf::calculate_dcf,
            commands::banks::calculate_banks,
            commands::rim::calculate_rim,
            commands::rnav::calculate_rnav,
            commands::projected_ceiling::calculate_projected_ceiling,
            commands::crypto_indicator::record_crypto_indicator,
            commands::crypto_indicator::list_crypto_indicators,
            commands::valuation::list_valuations,
            commands::valuation::get_valuation_inputs,
            commands::valuation::update_valuation,
            commands::valuation::delete_valuation,
            commands::alert_rule::create_alert_rule,
            commands::alert_rule::list_alert_rules,
            commands::alert_rule::set_alert_rule_active,
            commands::alert_rule::delete_alert_rule,
            commands::api_key::create_api_key,
            commands::api_key::list_api_keys,
            commands::api_key::rename_api_key,
            commands::api_key::delete_api_key,
            commands::chat::ask_ai,
            commands::document_extraction::extract_document_data,
            commands::conversation::list_conversations,
            commands::conversation::create_conversation,
            commands::conversation::rename_conversation,
            commands::conversation::delete_conversation,
            commands::conversation::get_conversation_messages,
            commands::conversation::send_conversation_message,
            commands::ai_proposal::respond_to_valuation_proposal,
            commands::collector::run_stock_collector,
            commands::collector::run_crypto_collector,
            commands::collector::list_stock_quotes,
            commands::collector::list_stock_fundamentals,
            commands::collector::list_stock_dividends_avg,
            commands::collector::list_stock_dcf_fundamentals,
            commands::collector::list_stock_technicals,
            commands::collector::list_stock_dividend_payments,
            commands::collector::run_price_history_backfill,
            commands::collector::list_stock_price_history,
            commands::stock_notes::list_stock_notes,
            commands::stock_notes::save_stock_note,
            commands::truthid::test_truthid_connection,
            commands::truthid::send_test_sign_request,
            commands::truthid::create_cross_device_sign_request,
            commands::truthid::await_cross_device_sign_request_response,
            commands::truthid::update_sync_record,
            commands::truthid::pin_database_snapshot,
            commands::sync_registry::get_sync_record,
            commands::workspace::get_workspace,
            commands::workspace::list_workspaces,
            commands::workspace::create_workspace,
            commands::portfolio::list_portfolios,
            commands::portfolio::create_portfolio,
            commands::portfolio::rename_portfolio,
            commands::custodia::list_custodias,
            commands::custodia::create_custodia,
            commands::custodia::delete_custodia,
            commands::asset::list_assets,
            commands::asset::create_asset,
            commands::transaction::create_transaction,
            commands::transaction::delete_transaction,
            commands::transaction::list_transactions,
            commands::transaction::get_portfolio_positions,
            commands::profitability::get_portfolio_profitability,
            commands::watchlist::list_watchlists,
            commands::watchlist::create_watchlist,
            commands::watchlist::delete_watchlist,
            commands::watchlist::list_watchlist_items,
            commands::watchlist::add_watchlist_item,
            commands::watchlist::update_watchlist_item,
            commands::watchlist::remove_watchlist_item,
            commands::watchlist::list_favorite_assets,
            commands::watchlist::toggle_favorite,
            commands::thesis::list_theses,
            commands::thesis::create_thesis,
            commands::thesis::update_thesis,
            commands::thesis::delete_thesis,
            commands::thesis::list_thesis_attachments,
            commands::thesis::add_thesis_attachment,
            commands::thesis::delete_thesis_attachment,
            commands::thesis::get_thesis_attachment_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
