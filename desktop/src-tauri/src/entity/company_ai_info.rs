//! `SeaORM` Entity, hand-written to match the codegen style used elsewhere
//! in this module (see `stock_notes.rs`).

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "company_ai_info")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub ticker: String,
    pub asset_class: String,
    pub content: String,
    pub provider: String,
    pub model: String,
    pub generated_at: String,
    pub regenerate_reason: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
