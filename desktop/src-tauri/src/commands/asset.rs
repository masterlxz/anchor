use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
    Unchanged,
};
use serde::Deserialize;

use crate::entity::{assets, fii_cnpj_cache};
use crate::error::AppError;

/// Mantém `fii_cnpj_cache` (Sessão 43, usada pela tela de Pesquisa) em
/// sincronia sempre que `assets.cnpj` é escrito por aqui — sem isso, corrigir
/// o CNPJ de um FII em Ativos não apareceria na Pesquisa (ela lê o cache,
/// não `assets`), quebrando a expectativa do dono do projeto de que edição
/// numa tela reflita na outra. Find-then-write (sem upsert nativo do
/// SeaORM aqui) — mesmo estilo do resto do arquivo.
async fn upsert_fii_cnpj_cache(
    db: &DatabaseConnection,
    ticker: &str,
    cnpj: &str,
    fund_name: &str,
) -> Result<(), AppError> {
    let existing = fii_cnpj_cache::Entity::find()
        .filter(fii_cnpj_cache::Column::Ticker.eq(ticker))
        .one(db)
        .await?;

    let now = chrono::Utc::now().to_rfc3339();
    match existing {
        Some(row) => {
            fii_cnpj_cache::ActiveModel {
                id: Unchanged(row.id),
                cnpj: Set(cnpj.to_string()),
                fund_name: Set(fund_name.to_string()),
                resolved_at: Set(now),
                ..Default::default()
            }
            .update(db)
            .await?;
        }
        None => {
            fii_cnpj_cache::ActiveModel {
                ticker: Set(ticker.to_string()),
                cnpj: Set(cnpj.to_string()),
                fund_name: Set(fund_name.to_string()),
                resolved_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await?;
        }
    }

    Ok(())
}

// Fase 10.2, escopo Sessão 29 — as demais classes expandidas na Sessão 30
// (REIT, metal, imóvel, empresa não listada, ver PHASE.md item 8) ficam pra
// uma fatia futura, junto com as decisões em aberto delas. `fii` entrou na
// Sessão 41, `etf_br` na Sessão 49 (2026-08-02) — mesmo mecanismo de
// listagem B3 e mesmo endpoint Yahoo (`{ticker}.SA`) que `acao_br` já usa,
// sem coletor novo — ver `ASSET_CLASSES_WITH_AUTO_QUOTE` em
// `profitability.rs`. `etf_br` fica só na fatia de cotação por ora — a CVM
// não tem uma categoria de dados abertos própria pra fundo de índice
// (achado pesquisando de verdade, diferente de FII/FIDC/FIP, que têm; ver
// PHASE.md item 8), então sem indicador de fundo específico ainda. `cripto`
// entrou na Sessão 51 — cotação automática também, mas fonte própria
// (CoinGecko, não Yahoo `.SA` — cripto não é listada na B3) via
// `--crypto-ticker` no coletor Python (`commands/collector.rs`). `bdr`
// entrou na Sessão 53 — recibo negociado na B3 representando empresa
// estrangeira (ex.: AAPL34), mas bate no mesmo endpoint Yahoo `.SA` que
// `acao_br`/`fii`/`etf_br` já usam (confirmado ao vivo), zero coletor
// novo, mesmo caminho de `etf_br`: sem fundamentos (bolsai não tem). `metal`
// entrou na Sessão 55 — cotação automática também (só ouro, `XAU`, por ora),
// fonte própria (Yahoo sem `.SA` — contrato futuro do COMEX, `GC=F`, não é
// listado na B3) via `--metal-ticker` no coletor Python, preço convertido
// de onça troy pra grama na fonte (`sources/metais_yahoo.py`). `acao_internacional`
// (ação americana NYSE/NASDAQ) ganhou cotação automática numa fatia 1 —
// mesmo endpoint Yahoo `.SA` do resto, só sem o sufixo (ticker puro, ex.:
// AAPL), via `--us-ticker` no coletor Python; `acoes_yahoo.py` ganhou um
// parâmetro `suffix` (default `.SA`) pra isso em vez de duplicar o módulo.
// Fatia 2 acrescentou fundamentos + DCF via SEC EDGAR (`sources/sec_edgar.py`)
// nas mesmas tabelas `stock_fundamentals`/`stock_dcf_fundamentals` que
// `acao_br` já usa (schema já era agnóstico de moeda/classe, sem migration).

const ASSET_CLASSES: [&str; 11] = [
    "acao_br",
    "fii",
    "etf_br",
    "cripto",
    "bdr",
    "metal",
    "acao_internacional",
    "reit",
    "etf_us",
    "tesouro_direto",
    "renda_fixa",
];
const EXPOSURE_TYPES: [&str; 2] = ["pais", "categoria_especial"];

#[tauri::command]
pub async fn list_assets(
    db: tauri::State<'_, DatabaseConnection>,
) -> Result<Vec<assets::Model>, AppError> {
    Ok(assets::Entity::find()
        .order_by_asc(assets::Column::Ticker)
        .all(db.inner())
        .await?)
}

#[derive(Deserialize)]
pub struct CreateAssetRequest {
    pub ticker: String,
    pub name: String,
    pub asset_class: String,
    pub currency: String,
    pub exchange: Option<String>,
    pub exposure_type: String,
    pub exposure_value: String,
    // Fase 10, item 8, Sessão 41 — só relevante pra `fii` por ora (chave de
    // junção com `fii_cvm_monthly`/`fii_cvm_properties`, resolvida no
    // frontend via `resolve_fii_cnpj` e confirmada pelo usuário antes do
    // submit). `None` pras demais classes, e também pro próprio FII se o
    // usuário não confirmar nenhuma sugestão.
    pub cnpj: Option<String>,
}

#[tauri::command]
pub async fn create_asset(
    db: tauri::State<'_, DatabaseConnection>,
    request: CreateAssetRequest,
) -> Result<assets::Model, AppError> {
    if !ASSET_CLASSES.contains(&request.asset_class.as_str()) {
        return Err(AppError::InvalidGuard(format!(
            "asset_class '{}' not supported yet (expected one of {ASSET_CLASSES:?})",
            request.asset_class
        )));
    }
    if !EXPOSURE_TYPES.contains(&request.exposure_type.as_str()) {
        return Err(AppError::InvalidGuard(format!(
            "exposure_type '{}' invalid (expected 'pais' or 'categoria_especial')",
            request.exposure_type
        )));
    }

    let asset = assets::ActiveModel {
        ticker: Set(request.ticker.clone()),
        name: Set(request.name.clone()),
        asset_class: Set(request.asset_class),
        currency: Set(request.currency),
        exchange: Set(request.exchange),
        exposure_type: Set(request.exposure_type),
        exposure_value: Set(request.exposure_value),
        cnpj: Set(request.cnpj.clone()),
        created_at: Set(chrono::Utc::now().to_rfc3339()),
        ..Default::default()
    }
    .insert(db.inner())
    .await?;

    if let Some(cnpj) = request.cnpj {
        upsert_fii_cnpj_cache(db.inner(), &request.ticker, &cnpj, &request.name).await?;
    }

    Ok(asset)
}

#[derive(Deserialize)]
pub struct UpdateAssetCnpjRequest {
    pub asset_id: i32,
    pub cnpj: String,
}

/// Comando estreito (não um `update_asset` genérico) — só existe pro caso
/// de `resolve_fii_cnpj` não achar sugestão nenhuma no cadastro (ou o
/// usuário não confirmar), deixando `assets.cnpj` nulo. Sem isso não
/// haveria jeito de colar o CNPJ manualmente depois, já que `assets` não
/// tem nenhum outro comando de edição (catálogo pensado como append-only na
/// Fase 10.2). Mesmo padrão find-by-id→`NotFound`→`Unchanged` de
/// `rename_portfolio`.
#[tauri::command]
pub async fn update_asset_cnpj(
    db: tauri::State<'_, DatabaseConnection>,
    request: UpdateAssetCnpjRequest,
) -> Result<assets::Model, AppError> {
    let existing = assets::Entity::find_by_id(request.asset_id)
        .one(db.inner())
        .await?
        .ok_or_else(|| AppError::NotFound(format!("asset {}", request.asset_id)))?;

    let updated = assets::ActiveModel {
        id: Unchanged(existing.id),
        cnpj: Set(Some(request.cnpj.clone())),
        ..Default::default()
    }
    .update(db.inner())
    .await?;

    upsert_fii_cnpj_cache(db.inner(), &existing.ticker, &request.cnpj, &existing.name).await?;

    Ok(updated)
}
