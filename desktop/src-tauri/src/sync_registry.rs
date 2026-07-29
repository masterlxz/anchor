use alloy_primitives::{Address, FixedBytes};
use alloy_sol_types::{sol, SolCall, SolError};
use serde_json::json;

use crate::error::AppError;

// Interface ABI do `contracts/src/SyncRegistry.sol` — leitura (`getRecord`) e,
// desde a Fase 8.2, escrita (`updateRecord`, calldata montado aqui e assinado
// via o canal delegado do TruthID em `commands/truthid.rs`, não por este
// módulo, que nunca fala com uma chave privada).
sol! {
    struct CidRecord {
        string cid;
        bytes32 contentHash;
        uint256 updatedAt;
        uint256 version;
        bool exists;
    }

    function getRecord(address who) external view returns (CidRecord memory);
    function updateRecord(string calldata cid, bytes32 contentHash) external;

    error RecordNotFound(address who);
}

/// RPC pública da Base Sepolia (testnet) — sem chave, mesmo endpoint já usado
/// como fallback pelo resto do ecossistema TruthID (`desktop/src/config/wagmi.ts`
/// de lá). Base Mainnet só entra quando o fluxo completo da Fase 8 estiver
/// provado ponta a ponta no testnet.
const RPC_URL: &str = "https://sepolia.base.org";

/// Deployado em Base Sepolia na Sessão 30 (`forge script script/DeploySyncRegistry.s.sol
/// --broadcast --ledger`, tx `0x52b347795c113a30cfac614497d43c62d1efce6a3c2b3cb7990d2a888aa4bfae`,
/// ver Fase 8.1 em `project/PHASE.md`).
pub(crate) const SYNC_REGISTRY_ADDRESS: &str = "0x9e2fd1a4146e3c40ecfef9d35a09e75e00ac30e8";

/// Busca o registro de sync de um endereço via `eth_call` público — não
/// precisa de assinatura, qualquer RPC serve. Retorna `None` (não `Err`)
/// quando o contrato reverte com `RecordNotFound`, já que "não tem registro
/// ainda" é um resultado esperado, não uma falha.
pub async fn get_record(who: Address) -> Result<Option<CidRecord>, AppError> {
    let calldata = getRecordCall { who }.abi_encode();

    let client = reqwest::Client::new();
    let response: serde_json::Value = client
        .post(RPC_URL)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_call",
            "params": [
                { "to": SYNC_REGISTRY_ADDRESS, "data": format!("0x{}", hex::encode(&calldata)) },
                "latest"
            ]
        }))
        .send()
        .await?
        .json()
        .await?;

    if let Some(error) = response.get("error") {
        // Reverte com `RecordNotFound(address)` → "ainda não tem registro",
        // não é um erro de verdade. Qualquer outro revert/erro de RPC propaga.
        let revert_data = error
            .get("data")
            .and_then(|d| d.as_str())
            .and_then(|hex_str| hex::decode(hex_str.trim_start_matches("0x")).ok());
        if let Some(bytes) = revert_data {
            if RecordNotFound::abi_decode(&bytes).is_ok() {
                return Ok(None);
            }
        }

        let message = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown RPC error");
        return Err(AppError::Rpc(message.to_string()));
    }

    let result_hex = response
        .get("result")
        .and_then(|r| r.as_str())
        .ok_or_else(|| AppError::Rpc("RPC response missing 'result' field".to_string()))?;
    let bytes = hex::decode(result_hex.trim_start_matches("0x"))
        .map_err(|e| AppError::Rpc(format!("invalid hex in RPC result: {e}")))?;

    let record = getRecordCall::abi_decode_returns(&bytes)
        .map_err(|e| AppError::Rpc(format!("failed to decode contract return value: {e}")))?;

    Ok(Some(record))
}

pub fn parse_address(raw: &str) -> Result<Address, AppError> {
    raw.parse()
        .map_err(|_| AppError::InvalidAddress(raw.to_string()))
}

/// Parseia um content hash em hex (`0x` + 64 chars, ou só os 64 chars) pro
/// `bytes32` que o contrato espera. Mesmo espírito de validação de
/// `parse_address`, mas pra um campo de tamanho fixo em vez de um endereço.
pub fn parse_content_hash(raw: &str) -> Result<FixedBytes<32>, AppError> {
    let hex_str = raw.trim().trim_start_matches("0x");
    let bytes =
        hex::decode(hex_str).map_err(|_| AppError::InvalidInput(format!("invalid hex: {raw}")))?;
    let array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| AppError::InvalidInput(format!("content hash must be 32 bytes: {raw}")))?;
    Ok(FixedBytes::from(array))
}

/// Monta o calldata de `updateRecord(cid, contentHash)` — só codifica, não
/// fala com rede nem assina nada; quem executa de fato é o TruthID via
/// `/truthid/v1/sign-request` (`commands/truthid.rs::update_sync_record`).
pub fn build_update_record_calldata(cid: &str, content_hash: FixedBytes<32>) -> Vec<u8> {
    updateRecordCall { cid: cid.to_string(), contentHash: content_hash }.abi_encode()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_sol_types::SolValue;

    // Vetor de teste "à mão" — sem rede: confirma que o calldata montado pro
    // eth_call tem a forma esperada (seletor de 4 bytes + address em 32,
    // padding à esquerda) antes de confiar nisso contra o contrato real.
    #[test]
    fn get_record_calldata_has_selector_plus_padded_address() {
        let who: Address = "0x000000000000000000000000000000000000dEaD".parse().unwrap();
        let calldata = getRecordCall { who }.abi_encode();

        assert_eq!(calldata.len(), 4 + 32);
        assert_eq!(&calldata[4..16], &[0u8; 12]); // padding do address
        assert_eq!(&calldata[16..], who.as_slice());
    }

    #[test]
    fn decodes_a_full_cid_record_return_value() {
        let record = CidRecord {
            cid: "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi".to_string(),
            contentHash: [0x11u8; 32].into(),
            updatedAt: alloy_primitives::U256::from(1_752_600_000u64),
            version: alloy_primitives::U256::from(3u64),
            exists: true,
        };

        // Round-trip: codifica o valor de retorno como o contrato faria e
        // confirma que `abi_decode_returns` (usado de verdade em `get_record`)
        // recompõe exatamente os mesmos campos.
        let encoded = record.abi_encode();
        let decoded = getRecordCall::abi_decode_returns(&encoded).unwrap();

        assert_eq!(decoded.cid, record.cid);
        assert_eq!(decoded.contentHash, record.contentHash);
        assert_eq!(decoded.updatedAt, record.updatedAt);
        assert_eq!(decoded.version, record.version);
        assert_eq!(decoded.exists, record.exists);
    }

    #[test]
    fn parse_address_rejects_garbage() {
        assert!(parse_address("not-an-address").is_err());
    }

    #[test]
    fn parse_address_accepts_checksummed_and_lowercase() {
        assert!(parse_address("0x000000000000000000000000000000000000dEaD").is_ok());
        assert!(parse_address("0x000000000000000000000000000000000000dead").is_ok());
    }

    #[test]
    fn parse_content_hash_accepts_with_and_without_0x_prefix() {
        let hex_64_chars = "11".repeat(32);
        let expected = FixedBytes::from([0x11u8; 32]);
        assert_eq!(parse_content_hash(&format!("0x{hex_64_chars}")).unwrap(), expected);
        assert_eq!(parse_content_hash(&hex_64_chars).unwrap(), expected);
    }

    #[test]
    fn parse_content_hash_rejects_wrong_length_or_garbage() {
        assert!(parse_content_hash("0x1234").is_err());
        assert!(parse_content_hash("not-hex-at-all-not-hex-at-all-not-hex-at-all-not-hex-1").is_err());
    }

    #[test]
    fn update_record_calldata_has_selector_and_encodes_both_args() {
        let content_hash = FixedBytes::from([0x22u8; 32]);
        let calldata = build_update_record_calldata("bafybeitestcid", content_hash);

        // Seletor de 4 bytes de `updateRecord(string,bytes32)` + o resto é o
        // ABI encoding padrão (offset da string dinâmica + bytes32 fixo +
        // length/bytes da string) — round-trip via decode confirma a forma.
        assert_eq!(&calldata[..4], &updateRecordCall::SELECTOR);
        let decoded = updateRecordCall::abi_decode(&calldata).unwrap();
        assert_eq!(decoded.cid, "bafybeitestcid");
        assert_eq!(decoded.contentHash, content_hash);
    }
}
