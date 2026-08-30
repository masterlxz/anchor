use std::collections::HashMap;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};

use crate::commands::api_key::read_api_key_secret;
use crate::commands::document_extraction::{extract_via_claude, extract_via_gemini, extract_via_openai};
use crate::domain::chat_provider::Provider;
use crate::domain::cvm_dividend_notice::{resolve_notices, ExtractedNotice, ResolvedNotice};
use crate::domain::dividend_suggestion::{quantity_held_by, LedgerRow};
use crate::entity::{assets, cvm_dividend_notice_documents, portfolio, suggested_dividends, transactions};
use crate::error::AppError;
use crate::finance_api::{client, stocks as finance_api_stocks, FinanceApiHandle};

// Fase 13.6 — parte "futuro": "Relatório Proventos" da CVM (formulário
// padronizado, achado ao vivo nesta sessão — não Fato Relevante/Comunicado
// ao Mercado em texto livre) via `easybusiness` (`cvm_ipe.py`), PDF extraído
// por IA (reaproveita `commands::document_extraction`'s 3 funções por
// provider). Sugestão vira `suggested_dividends` com `source = "cvm"` —
// mesmo fluxo de revisão/confirmação humana que já existe pro "passado"
// (Yahoo). Decisão do dono do projeto: se um documento trouxer valores
// divergentes entre classes de ação (ON/PN/units) sem dar pra casar o ISIN
// certo com o ticker, a sugestão automática é pulada (fica só no ledger de
// auditoria) — nunca arrisca um valor errado.

const STATUS_PENDING: &str = "pending";
const SOURCE_CVM: &str = "cvm";

const EXTRACTION_INSTRUCTIONS: &str = "This PDF is a CVM (Brazilian securities \
regulator) \"Relatório Proventos\" filing — a standardized form a public company \
files to announce a dividend or JCP (juros sobre capital próprio) payment. It can \
list more than one share class (ON/PN/units), each with its own ISIN and its own \
gross value per share. Extract every share-class row from the \"Valor Bruto \
(R$/Unidade)\" table. For each row return: isin (the ISIN code), ticker_class \
(\"ON\", \"PN\", \"UNIT\", or \"OTHER\" if unclear), label (\"DIVIDENDO\" or \"JCP\" \
— read from the \"Ato Societário\"/form header, not guessed from the amount), \
rate_per_share (the gross value per share/unit as a plain number, no currency \
symbol), payment_date (\"Data Pagamento\", format YYYY-MM-DD), com_date (\"Último \
dia de negociação com Direitos\", format YYYY-MM-DD — the ex-dividend/Data Com \
date), approved_on (\"Data Aprovação\", format YYYY-MM-DD). Use null for any field \
genuinely absent from the document — never guess a value.";

fn extraction_json_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "notices": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "isin": {"type": ["string", "null"]},
                        "ticker_class": {"type": ["string", "null"]},
                        "label": {"type": ["string", "null"]},
                        "rate_per_share": {"type": ["number", "null"]},
                        "payment_date": {"type": ["string", "null"]},
                        "com_date": {"type": ["string", "null"]},
                        "approved_on": {"type": ["string", "null"]}
                    },
                    "required": [
                        "isin", "ticker_class", "label", "rate_per_share", "payment_date",
                        "com_date", "approved_on"
                    ],
                    "additionalProperties": false
                }
            }
        },
        "required": ["notices"],
        "additionalProperties": false
    })
}

#[derive(Deserialize)]
struct ExtractionResult {
    notices: Vec<ExtractedNotice>,
}

/// Distingue os 2 jeitos de "não virou sugestão": `Unusable` é permanente —
/// grava no ledger com o motivo e nunca reprocessa (documento sem linha
/// utilizável, linhas divergentes, ou a CVM devolvendo algo que nem é PDF —
/// achado ao vivo: acontece de verdade pra alguns `numVersao` antigos,
/// página de erro HTML no lugar do arquivo). `Resolved` segue pro chamador
/// decidir se `payment_date` já passou (também não vira sugestão, mas por
/// motivo diferente — coberto pelo fluxo Yahoo).
enum NoticeOutcome {
    Resolved(ResolvedNotice),
    Unusable(String),
}

/// Baixa o PDF e extrai via IA. `Err` fica só pra falha de rede/IA de
/// verdade (timeout, 503 "alta demanda") — tratada pelo chamador como
/// transitória, sem gravar no ledger, pra poder tentar de nascer num
/// "Check CVM notices" futuro em vez de ficar pulada pra sempre.
async fn process_notice(
    provider: Provider,
    api_key: &str,
    model: &str,
    pdf_url: &str,
) -> Result<NoticeOutcome, AppError> {
    let http = reqwest::Client::new();
    let bytes = http.get(pdf_url).send().await?.bytes().await?;
    if !bytes.starts_with(b"%PDF-") {
        return Ok(NoticeOutcome::Unusable(
            "a CVM não devolveu um PDF válido pra esse documento (achado ao vivo: acontece com \
             algumas versões antigas de filing — provável falha do lado deles, não deste app)"
                .to_string(),
        ));
    }
    let pdf_base64 = STANDARD.encode(&bytes);
    let schema = extraction_json_schema();

    let raw = match provider {
        Provider::Gemini => {
            extract_via_gemini(api_key, model, &pdf_base64, EXTRACTION_INSTRUCTIONS, &schema).await?
        }
        Provider::Claude => {
            extract_via_claude(api_key, model, &pdf_base64, EXTRACTION_INSTRUCTIONS, &schema).await?
        }
        Provider::OpenAi => {
            extract_via_openai(api_key, model, &pdf_base64, EXTRACTION_INSTRUCTIONS, &schema).await?
        }
    };

    let result: ExtractionResult = serde_json::from_value(raw)
        .map_err(|e| AppError::InvalidInput(format!("unexpected extraction shape: {e}")))?;

    Ok(match resolve_notices(result.notices) {
        Some(resolved) => NoticeOutcome::Resolved(resolved),
        None => NoticeOutcome::Unusable(
            "sem linha utilizável, ou valores divergentes entre classes de ação".to_string(),
        ),
    })
}

#[derive(Deserialize)]
pub struct CheckCvmDividendNoticesRequest {
    pub portfolio_id: i32,
    pub key_id: i32,
    pub model: String,
}

#[derive(Serialize)]
pub struct CheckCvmDividendNoticesResult {
    pub documents_checked: usize,
    pub suggestions_created: usize,
    pub skipped: usize,
}

#[tauri::command]
pub async fn check_cvm_dividend_notices(
    db: tauri::State<'_, DatabaseConnection>,
    finance_api: tauri::State<'_, FinanceApiHandle>,
    request: CheckCvmDividendNoticesRequest,
) -> Result<CheckCvmDividendNoticesResult, AppError> {
    run_check(db.inner(), finance_api.inner(), request).await
}

/// Corpo de verdade do comando, separado do wrapper `#[tauri::command]` só
/// pra ser testável direto contra `&DatabaseConnection`/`&FinanceApiHandle`
/// sem precisar fabricar `tauri::State` (mesma separação que
/// `commands::collector::run_stock_collector` já usa em cima de
/// `finance_api::stocks::collect_quotes` e companhia).
async fn run_check(
    db: &DatabaseConnection,
    finance_api: &FinanceApiHandle,
    request: CheckCvmDividendNoticesRequest,
) -> Result<CheckCvmDividendNoticesResult, AppError> {
    portfolio::Entity::find_by_id(request.portfolio_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("portfolio {}", request.portfolio_id)))?;

    let (provider, api_key) = read_api_key_secret(db, request.key_id).await?;

    let stock_assets: Vec<assets::Model> = assets::Entity::find()
        .filter(assets::Column::AssetClass.eq("acao_br"))
        .all(db)
        .await?;

    let txs = transactions::Entity::find()
        .filter(transactions::Column::PortfolioId.eq(request.portfolio_id))
        .all(db)
        .await?;
    let mut txs_by_asset: HashMap<i32, Vec<LedgerRow>> = HashMap::new();
    for tx in &txs {
        if let Some(asset_id) = tx.asset_id {
            txs_by_asset.entry(asset_id).or_default().push(LedgerRow {
                transaction_type: tx.transaction_type.clone(),
                quantity: tx.quantity.unwrap_or(0.0),
                transaction_date: tx.transaction_date.clone(),
            });
        }
    }

    let today = chrono::Local::now().date_naive().format("%Y-%m-%d").to_string();

    // Agrupa por `cvm_code` (não por ticker): uma "Relatório Proventos" vale
    // pra todas as classes de ação (ON/PN) do portfolio na mesma empresa. Só
    // resolve/gasta chamada de IA em ativo que o portfolio de fato tem
    // quantidade hoje — sem isso, `quantity_held_by` sempre daria 0 e a
    // sugestão nem seria criada mesmo assim.
    let mut asset_ids_by_cvm_code: HashMap<i32, Vec<i32>> = HashMap::new();
    for asset in &stock_assets {
        let Some(asset_txs) = txs_by_asset.get(&asset.id) else {
            continue;
        };
        if quantity_held_by(asset_txs, &today) <= 0.0 {
            continue;
        }
        let Some(cvm_code) = finance_api_stocks::resolve_cvm_code(finance_api, &asset.ticker).await?
        else {
            continue;
        };
        asset_ids_by_cvm_code.entry(cvm_code).or_default().push(asset.id);
    }

    let mut documents_checked = 0usize;
    let mut suggestions_created = 0usize;
    let mut skipped = 0usize;

    for (cvm_code, asset_ids) in asset_ids_by_cvm_code {
        let response = client::fetch_company_dividend_notices(finance_api, cvm_code).await?;

        for notice in response.data {
            let already_processed = cvm_dividend_notice_documents::Entity::find()
                .filter(cvm_dividend_notice_documents::Column::CvmCode.eq(cvm_code))
                .filter(
                    cvm_dividend_notice_documents::Column::ProtocoloEntrega
                        .eq(&notice.protocolo_entrega),
                )
                .one(db)
                .await?;
            if already_processed.is_some() {
                continue;
            }
            documents_checked += 1;

            let outcome =
                process_notice(provider, &api_key, &request.model, &notice.link_download).await;

            // Falha de rede/API (inclui CVM devolvendo uma página de erro
            // HTML no lugar do PDF, achado ao vivo nesta sessão contra um
            // `numVersao` antigo) não vira registro permanente no ledger —
            // diferente de uma ambiguidade genuína (valores divergentes,
            // `payment_date` passado), uma falha transitória (503 "alta
            // demanda", timeout) merece nova tentativa num "Check CVM
            // notices" futuro, não ficar pulada pra sempre.
            let outcome = match outcome {
                Ok(outcome) => outcome,
                Err(err) => {
                    // Transitório (rede, 503 "alta demanda") — não grava no
                    // ledger, fica elegível pra nova tentativa num "Check
                    // CVM notices" futuro em vez de pular pra sempre.
                    skipped += 1;
                    eprintln!(
                        "cvm_dividend_notice: falha ao processar {} — {err}",
                        notice.link_download
                    );
                    continue;
                }
            };

            // A CVM devolve o histórico inteiro de "Relatório Proventos",
            // não só os futuros — mas o "passado" já é coberto de forma
            // mais confiável pelo fluxo Yahoo (pagamento real, não só
            // aviso). Escopo desta automação é só o "futuro" (nome da
            // fatia, ver PHASE.md 13.6): um aviso com `payment_date` já
            // passado é permanente no ledger (não reprocessa à toa), mas
            // não vira sugestão — evita duplicar o que o "Generate
            // suggestions" (Yahoo) já resolve sozinho.
            let (matched, note, resolved) = match outcome {
                NoticeOutcome::Resolved(entry) if entry.payment_date.as_str() > today.as_str() => {
                    (true, None, Some(entry))
                }
                NoticeOutcome::Resolved(_) => (
                    false,
                    Some("payment_date já passou — coberto pelo fluxo Yahoo (passado)".to_string()),
                    None,
                ),
                NoticeOutcome::Unusable(reason) => (false, Some(reason), None),
            };

            if let Some(entry) = resolved {
                for &asset_id in &asset_ids {
                    let exists = suggested_dividends::Entity::find()
                        .filter(suggested_dividends::Column::PortfolioId.eq(request.portfolio_id))
                        .filter(suggested_dividends::Column::AssetId.eq(asset_id))
                        .filter(suggested_dividends::Column::PaymentDate.eq(&entry.payment_date))
                        .filter(suggested_dividends::Column::Source.eq(SOURCE_CVM))
                        .one(db)
                        .await?;
                    if exists.is_some() {
                        continue;
                    }

                    let Some(asset_txs) = txs_by_asset.get(&asset_id) else {
                        continue;
                    };
                    // Corte pela data do próprio aviso, não por hoje: a CVM
                    // devolve o histórico inteiro de "Relatório Proventos",
                    // não só os futuros — um aviso com `payment_date` no
                    // passado precisa da posição reconstruída naquela data
                    // (mesma regra do fluxo Yahoo), senão quem comprou depois
                    // de um provento antigo ganharia sugestão de dinheiro que
                    // nunca recebeu. Pra `payment_date` futuro isso equivale
                    // à posição de hoje (não há lançamento além de hoje pra
                    // excluir), então uma única regra cobre os dois casos.
                    let quantity = quantity_held_by(asset_txs, &entry.payment_date);
                    if quantity <= 0.0 {
                        continue;
                    }

                    suggested_dividends::ActiveModel {
                        portfolio_id: Set(request.portfolio_id),
                        asset_id: Set(asset_id),
                        payment_date: Set(entry.payment_date.clone()),
                        amount: Set(entry.rate_per_share),
                        quantity: Set(quantity),
                        total: Set(quantity * entry.rate_per_share),
                        status: Set(STATUS_PENDING.to_string()),
                        com_date: Set(entry.com_date.clone()),
                        source: Set(SOURCE_CVM.to_string()),
                        payment_type: Set(entry.payment_type.clone()),
                        created_at: Set(Utc::now().to_rfc3339()),
                        ..Default::default()
                    }
                    .insert(db)
                    .await?;
                    suggestions_created += 1;
                }
            } else {
                skipped += 1;
            }

            cvm_dividend_notice_documents::ActiveModel {
                cvm_code: Set(cvm_code),
                protocolo_entrega: Set(notice.protocolo_entrega),
                matched: Set(matched),
                note: Set(note),
                processed_at: Set(Utc::now().to_rfc3339()),
                ..Default::default()
            }
            .insert(db)
            .await?;
        }
    }

    Ok(CheckCvmDividendNoticesResult {
        documents_checked,
        suggestions_created,
        skipped,
    })
}

// Teste `#[ignore]` — mesma convenção de `finance_api::stocks::tests`:
// precisa da Finance API real em `http://localhost:8000`, do banco real de
// dev (`/data/anchor.db`, dentro do container do Anchor) e de uma
// `ai_api_key` real já cadastrada lá (`list_api_keys`). Roda contra dado
// real do dono do projeto (portfolio 1, BBAS3/cvm_code 1023, confirmado
// nesta sessão) — não fabrica portfolio/asset/transação. Rodar com
// `cargo test --lib -- --ignored cvm_dividend_notice`.
#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;

    async fn dev_db() -> DatabaseConnection {
        Database::connect("sqlite:///data/anchor.db?mode=rwc")
            .await
            .expect("failed to connect to the real dev database")
    }

    fn handle() -> FinanceApiHandle {
        FinanceApiHandle::for_test(
            "http://localhost:8000".to_string(),
            "local-dev-key-change-me".to_string(),
        )
    }

    #[tokio::test]
    #[ignore]
    async fn live_check_cvm_dividend_notices_is_idempotent() {
        let db = dev_db().await;
        let finance_api = handle();

        let key = crate::entity::ai_api_key::Entity::find()
            .one(&db)
            .await
            .unwrap()
            .expect("no ai_api_key configured in the real dev database");
        let portfolio_row = portfolio::Entity::find().one(&db).await.unwrap().expect("no portfolio");

        let request = CheckCvmDividendNoticesRequest {
            portfolio_id: portfolio_row.id,
            key_id: key.id,
            model: crate::domain::chat_provider::Provider::parse(&key.provider)
                .map(|p| match p {
                    Provider::Gemini => "gemini-3.1-flash-lite",
                    Provider::Claude => "claude-haiku-4-5",
                    Provider::OpenAi => "gpt-5-mini",
                })
                .unwrap()
                .to_string(),
        };

        let first = run_check(&db, &finance_api, request).await;
        // Não assume sucesso incondicional — só que não estoura, e imprime o
        // resultado real pra inspeção manual (dado real, não fixture).
        println!("first run: {:?}", first.as_ref().map(|r| (r.documents_checked, r.suggestions_created, r.skipped)));
        assert!(first.is_ok(), "first run failed: {:?}", first.err());

        let ledger_count_before = cvm_dividend_notice_documents::Entity::find().all(&db).await.unwrap().len();

        let key2 = crate::entity::ai_api_key::Entity::find().one(&db).await.unwrap().unwrap();
        let request2 = CheckCvmDividendNoticesRequest {
            portfolio_id: portfolio_row.id,
            key_id: key2.id,
            model: "gemini-3.1-flash-lite".to_string(),
        };
        let second = run_check(&db, &finance_api, request2).await.unwrap();
        assert_eq!(second.documents_checked, 0, "re-running must not reprocess the same documents");

        let ledger_count_after = cvm_dividend_notice_documents::Entity::find().all(&db).await.unwrap().len();
        assert_eq!(ledger_count_before, ledger_count_after, "re-running must not duplicate ledger rows");
    }
}
