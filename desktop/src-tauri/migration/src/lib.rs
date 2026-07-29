pub use sea_orm_migration::prelude::*;

mod m20260709_010051_create_valuation_and_bazin_inputs;
mod m20260709_152010_create_graham_inputs;
mod m20260709_153134_create_gordon_inputs;
mod m20260709_154525_create_dcf_inputs;
mod m20260709_155420_create_banks_inputs;
mod m20260709_160307_create_rnav_inputs;
mod m20260709_160929_create_projected_ceiling_inputs;
mod m20260709_212958_create_crypto_score_tables;
mod m20260709_232211_create_stock_quotes;
mod m20260710_115333_create_stock_fundamentals_and_dividends;
mod m20260710_132548_create_stock_dcf_fundamentals;
mod m20260710_134142_add_tax_rate_to_stock_dcf_fundamentals;
mod m20260710_220000_rename_avg_dividend5y_column;
mod m20260711_093000_create_alert_rule_table;
mod m20260711_171445_create_alert_event_table;
mod m20260712_220000_add_payout_to_stock_fundamentals;
mod m20260712_223000_create_rim_inputs;
mod m20260715_215835_create_stock_technicals;
mod m20260715_215836_create_stock_notes;
mod m20260715_230504_add_revenue_to_stock_dcf_fundamentals;
mod m20260715_232005_create_stock_dividend_payments;
mod m20260716_005530_create_ai_api_key;
mod m20260721_120000_create_ai_conversation_and_message;
mod m20260721_140000_add_token_usage_to_ai_message;
mod m20260721_150000_create_ai_valuation_proposal;
mod m20260728_103434_add_inventory_to_stock_dcf_fundamentals;
mod m20260729_090000_create_workspace_and_portfolio;
mod m20260729_090100_create_custodia;
mod m20260729_090200_create_assets;
mod m20260729_090300_create_transactions;
mod m20260729_150000_add_details_to_stock_quotes;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260709_010051_create_valuation_and_bazin_inputs::Migration),
            Box::new(m20260709_152010_create_graham_inputs::Migration),
            Box::new(m20260709_153134_create_gordon_inputs::Migration),
            Box::new(m20260709_154525_create_dcf_inputs::Migration),
            Box::new(m20260709_155420_create_banks_inputs::Migration),
            Box::new(m20260709_160307_create_rnav_inputs::Migration),
            Box::new(m20260709_160929_create_projected_ceiling_inputs::Migration),
            Box::new(m20260709_212958_create_crypto_score_tables::Migration),
            Box::new(m20260709_232211_create_stock_quotes::Migration),
            Box::new(m20260710_115333_create_stock_fundamentals_and_dividends::Migration),
            Box::new(m20260710_132548_create_stock_dcf_fundamentals::Migration),
            Box::new(m20260710_134142_add_tax_rate_to_stock_dcf_fundamentals::Migration),
            Box::new(m20260710_220000_rename_avg_dividend5y_column::Migration),
            Box::new(m20260711_093000_create_alert_rule_table::Migration),
            Box::new(m20260711_171445_create_alert_event_table::Migration),
            Box::new(m20260712_220000_add_payout_to_stock_fundamentals::Migration),
            Box::new(m20260712_223000_create_rim_inputs::Migration),
            Box::new(m20260715_215835_create_stock_technicals::Migration),
            Box::new(m20260715_215836_create_stock_notes::Migration),
            Box::new(m20260715_230504_add_revenue_to_stock_dcf_fundamentals::Migration),
            Box::new(m20260715_232005_create_stock_dividend_payments::Migration),
            Box::new(m20260716_005530_create_ai_api_key::Migration),
            Box::new(m20260721_120000_create_ai_conversation_and_message::Migration),
            Box::new(m20260721_140000_add_token_usage_to_ai_message::Migration),
            Box::new(m20260721_150000_create_ai_valuation_proposal::Migration),
            Box::new(m20260728_103434_add_inventory_to_stock_dcf_fundamentals::Migration),
            Box::new(m20260729_090000_create_workspace_and_portfolio::Migration),
            Box::new(m20260729_090100_create_custodia::Migration),
            Box::new(m20260729_090200_create_assets::Migration),
            Box::new(m20260729_090300_create_transactions::Migration),
            Box::new(m20260729_150000_add_details_to_stock_quotes::Migration),
        ]
    }
}
