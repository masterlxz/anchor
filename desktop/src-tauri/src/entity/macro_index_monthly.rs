//! `SeaORM` Entity, hand-written to match the generated style (see other
//! entities in this crate, e.g. `suggested_dividends.rs`).

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "macro_index_monthly")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub index_code: String,
    pub year_month: String,
    #[sea_orm(column_type = "Double")]
    pub value_pct: f64,
    pub source: String,
    pub fetched_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
