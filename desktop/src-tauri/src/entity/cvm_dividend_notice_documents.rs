//! `SeaORM` Entity, hand-written to match the generated style (see
//! `macro_index_monthly.rs` for the same reasoning — running
//! `sea-orm-cli generate entity` regenerates several unrelated existing
//! entities with a different naming convention, see Sessão 90).

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "cvm_dividend_notice_documents")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub cvm_code: i32,
    pub protocolo_entrega: String,
    pub matched: bool,
    pub note: Option<String>,
    pub processed_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
