//! `SeaORM` Entity — Fase 12, split juros/amortização de uma parcela gerada
//! de `Liability`. O registro de fluxo de caixa de fato é o
//! `general_transaction` linkado — esta tabela só guarda o detalhamento.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "liability_installments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub liability_id: i32,
    pub general_transaction_id: i32,
    pub numero_parcela: i32,
    #[sea_orm(column_type = "Double")]
    pub valor_parcela: f64,
    #[sea_orm(column_type = "Double")]
    pub valor_juros: f64,
    #[sea_orm(column_type = "Double")]
    pub valor_amortizacao: f64,
    #[sea_orm(column_type = "Double")]
    pub saldo_devedor_apos: f64,
    pub data_vencimento: String,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::liability::Entity",
        from = "Column::LiabilityId",
        to = "super::liability::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Liability,
    #[sea_orm(
        belongs_to = "super::general_transaction::Entity",
        from = "Column::GeneralTransactionId",
        to = "super::general_transaction::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    GeneralTransaction,
}

impl Related<super::liability::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Liability.def()
    }
}

impl Related<super::general_transaction::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GeneralTransaction.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
