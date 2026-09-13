use crate::error::AppError;

/// Fase 8, item 3 da fila (`project/PHASE.md`) — leitura pública de conteúdo pinado em Arweave
/// pelo TruthID (`/truthid/v1/pin`, migrado de IPFS na Sessão 87). Diferente da escrita (que passa
/// pelo canal delegado do TruthID), a leitura é só um `GET` público — Arweave não exige
/// autenticação nem TruthID no meio pra servir conteúdo já minerado.
const ARWEAVE_GATEWAY: &str = "https://arweave.net";

/// Busca os bytes crus de uma transação Arweave pelo `tx_id` (a parte depois de `ar://` no `cid`
/// gravado no `SyncRegistry`). Devolve o conteúdo exatamente como foi pinado — ainda cifrado, se
/// foi pinado pela Fase 8.4 (`sync_snapshot_cipher`) — decifrar é responsabilidade de quem chama.
pub async fn fetch_content(tx_id: &str) -> Result<Vec<u8>, AppError> {
    let url = format!("{ARWEAVE_GATEWAY}/{tx_id}");
    let response = reqwest::get(&url).await?;

    if !response.status().is_success() {
        return Err(AppError::Arweave(format!(
            "gateway returned {} for tx '{tx_id}'",
            response.status()
        )));
    }

    Ok(response.bytes().await?.to_vec())
}
