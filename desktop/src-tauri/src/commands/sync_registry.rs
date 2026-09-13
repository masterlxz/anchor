use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::arweave;
use crate::error::AppError;
use crate::sync_registry;
use crate::sync_snapshot_cipher;

use super::truthid::derive_sync_snapshot_key_loopback;

// Tipo próprio pro Tauri/JS em vez de expor o `CidRecord` gerado pela macro
// `sol!` diretamente — mesma razão do split `TruthIdWireResult`/
// `TruthIdSignResult` em `commands/truthid.rs`: os tipos ABI (`U256`,
// `FixedBytes<32>`) não implementam `serde::Serialize` e não são o formato
// que o frontend quer (strings simples).
#[derive(Serialize)]
pub struct CidRecordResponse {
    pub cid: String,
    pub content_hash: String,
    pub updated_at: u64,
    pub version: u64,
    pub exists: bool,
}

impl From<sync_registry::CidRecord> for CidRecordResponse {
    fn from(record: sync_registry::CidRecord) -> Self {
        CidRecordResponse {
            cid: record.cid,
            content_hash: record.contentHash.to_string(),
            updated_at: record.updatedAt.to::<u64>(),
            version: record.version.to::<u64>(),
            exists: record.exists,
        }
    }
}

/// Lê o registro de sync de um endereço via `eth_call` público — prova de
/// conceito da Fase 8.1 (leitura), sem escrita ainda (Fase 8.2). `None` tanto
/// quando o endereço nunca gravou nada quanto (por enquanto) quando o
/// contrato ainda não foi deployado de verdade.
#[tauri::command]
pub async fn get_sync_record(address: String) -> Result<Option<CidRecordResponse>, AppError> {
    let who = sync_registry::parse_address(&address)?;
    let record = sync_registry::get_record(who).await?;

    Ok(record.map(CidRecordResponse::from))
}

#[derive(Serialize)]
pub struct PulledSnapshotSummary {
    pub tx_id: String,
    pub record: CidRecordResponse,
    pub raw_content_hash_matches: bool,
    pub decrypted_bytes: u64,
}

// Fase 8, item 3 da fila (`project/PHASE.md`) — busca o conteúdo Arweave apontado pelo registro
// on-chain, decifra com a mesma chave determinística da Fase 8.4 e confere os dois checks de
// integridade (hash on-chain do conteúdo bruto + tag de autenticação do AES-GCM na decifra).
// Deliberadamente não restaura/sobrescreve o banco local — isso é escopo da Fase 8.5 (merge por
// replay causal), que ainda nem começou; só devolve um resumo pra preview na UI.
#[tauri::command]
pub async fn pull_and_verify_sync_snapshot(
    address: String,
) -> Result<PulledSnapshotSummary, AppError> {
    let who = sync_registry::parse_address(&address)?;
    let record = sync_registry::get_record(who)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("sync record for {address}")))?;

    let tx_id = sync_registry::parse_arweave_tx_id(&record.cid)?.to_string();
    let raw = arweave::fetch_content(&tx_id).await?;

    let mut hasher = Sha256::new();
    hasher.update(&raw);
    let computed_hash = format!("0x{}", hex::encode(hasher.finalize()));
    let response = CidRecordResponse::from(record);
    let raw_content_hash_matches = computed_hash.eq_ignore_ascii_case(&response.content_hash);

    let key = derive_sync_snapshot_key_loopback().await?;
    let decrypted = sync_snapshot_cipher::decrypt(&raw, &key)?;

    Ok(PulledSnapshotSummary {
        tx_id,
        record: response,
        raw_content_hash_matches,
        decrypted_bytes: decrypted.len() as u64,
    })
}
