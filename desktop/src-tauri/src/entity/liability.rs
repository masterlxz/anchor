//! `SeaORM` Entity — Fase 12, dívida/passivo com parcelas geradas
//! automaticamente (Price/SAC).

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "liabilities")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub workspace_id: i32,
    pub bank_account_id: i32,
    pub linked_asset_id: Option<i32>,
    pub nome: String,
    pub titular: String,
    #[sea_orm(column_type = "Double")]
    pub principal: f64,
    #[sea_orm(column_type = "Double")]
    pub taxa_percentual: f64,
    pub indexador: String,
    pub prazo_meses: i32,
    pub sistema_amortizacao: String,
    pub data_inicio: String,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::bank_account::Entity",
        from = "Column::BankAccountId",
        to = "super::bank_account::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    BankAccount,
    #[sea_orm(
        belongs_to = "super::assets::Entity",
        from = "Column::LinkedAssetId",
        to = "super::assets::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Assets,
    #[sea_orm(
        belongs_to = "super::workspace::Entity",
        from = "Column::WorkspaceId",
        to = "super::workspace::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Workspace,
}

impl Related<super::bank_account::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BankAccount.def()
    }
}

impl Related<super::assets::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Assets.def()
    }
}

impl Related<super::workspace::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Workspace.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
