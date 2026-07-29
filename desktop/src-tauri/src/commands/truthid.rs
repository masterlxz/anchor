use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::db;
use crate::dead_drop;
use crate::ecies;
use crate::error::AppError;
use crate::lan_sweep;
use crate::sync_registry;

/// Mesma faixa de portas que o TruthID Desktop tenta em
/// `desktop/src-tauri/src/local_signer_server.rs` (bloco próprio, longe do
/// LAN da Fase 13.9 e do Vite dev server) — precisa ser espelhada manualmente
/// aqui, é a única forma de descoberta (localhost, sem mDNS/broadcast).
const CANDIDATE_PORTS: [u16; 5] = [47950, 47951, 47952, 47953, 47954];

const APP_NAME: &str = "Practice Valuation";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Endereço de burn — dest do sign-request de teste. value="0" e callData="0x"
/// (transferência de valor puro, sem chamar nenhuma função) fazem desta prova
/// de conceito uma UserOperation real (assinada, enviada ao bundler, com
/// userOpHash/transactionHash de verdade) sem nenhum efeito econômico.
const TEST_DEST_ADDRESS: &str = "0x000000000000000000000000000000000000dEaD";

/// Como callData é vazio, o seletor calculado a partir desta assinatura nunca
/// vai bater com os 4 primeiros bytes do callData — a tela de aprovação do
/// TruthID mostra "não verificado" + bytes crus, o comportamento correto pra
/// uma transferência sem chamada de função (não é bug desta fatia).
const TEST_FUNCTION_SIGNATURE: &str = "practiceValuationTestPing()";

#[derive(Deserialize)]
struct PingResponse {
    version: String,
}

#[derive(Deserialize)]
struct HandshakeResponse {
    accepted: bool,
    error: Option<String>,
}

/// Tenta cada porta candidata em ordem até achar um TruthID Desktop
/// respondendo — não há descoberta de rede real, é tudo loopback.
async fn discover() -> Result<(u16, String), AppError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()?;

    for port in CANDIDATE_PORTS {
        let url = format!("http://127.0.0.1:{port}/truthid/v1/ping");
        if let Ok(resp) = client.get(&url).send().await {
            if let Ok(body) = resp.json::<PingResponse>().await {
                return Ok((port, body.version));
            }
        }
    }

    Err(AppError::TruthIdNotFound)
}

#[derive(Serialize)]
pub struct TruthIdHandshakeResult {
    port: u16,
    desktop_version: String,
    accepted: bool,
}

#[tauri::command]
pub async fn test_truthid_connection() -> Result<TruthIdHandshakeResult, AppError> {
    let (port, desktop_version) = discover().await?;

    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/truthid/v1/handshake");
    let body: HandshakeResponse = client
        .post(&url)
        .json(&serde_json::json!({ "appName": APP_NAME, "appVersion": APP_VERSION }))
        .send()
        .await?
        .json()
        .await?;

    if !body.accepted {
        return Err(AppError::TruthId(
            body.error.unwrap_or_else(|| "handshake rejected".to_string()),
        ));
    }

    Ok(TruthIdHandshakeResult { port, desktop_version, accepted: true })
}

// Formato de fio (o JSON que chega de fora: `SignRequestResponse` do TruthID
// Desktop via loopback, `desktop/src-tauri/src/sign_request.rs`, e o
// resultado que o Mobile entrega via LAN/dead-drop cross-device,
// `sign_request_approval_screen.dart::_deliver`) — sempre camelCase
// (`userOpHash`/`transactionHash`). Convertido pra `TruthIdSignResult`
// antes de voltar pro frontend via Tauri: **achado real desta sessão** —
// dar `rename_all = "camelCase"` direto em `TruthIdSignResult` (como uma
// sessão anterior tinha feito) também muda a serialização de volta pro
// Tauri/JS, mas o frontend (`TruthIdPanel.tsx`) lê `user_op_hash`/
// `transaction_hash` em snake_case (mesmo padrão que `TruthIdHandshakeResult`
// já usa) — os dois lados desta struct (desserializar o JSON alheio vs
// serializar de volta pro Tauri) precisam de convenções de nome diferentes,
// por isso são dois tipos diferentes agora. Só foi pego testando de
// verdade com um celular físico: `cargo test`/`tsc` nunca teriam pego,
// porque nenhum dos dois lados sozinho está "errado" — só a combinação dos
// dois é que quebra silenciosamente (os campos são `Option<T>`, então uma
// chave ausente nunca gera erro de parse, só vira `None`/`undefined`).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TruthIdWireResult {
    status: String,
    user_op_hash: Option<String>,
    transaction_hash: Option<String>,
    error: Option<String>,
}

impl From<TruthIdWireResult> for TruthIdSignResult {
    fn from(wire: TruthIdWireResult) -> Self {
        TruthIdSignResult {
            status: wire.status,
            user_op_hash: wire.user_op_hash,
            transaction_hash: wire.transaction_hash,
            error: wire.error,
        }
    }
}

#[derive(Serialize)]
pub struct TruthIdSignResult {
    status: String,
    user_op_hash: Option<String>,
    transaction_hash: Option<String>,
    error: Option<String>,
}

#[tauri::command]
pub async fn send_test_sign_request() -> Result<TruthIdSignResult, AppError> {
    let (port, _) = discover().await?;

    // Margem sobre o timeout de 5min que o próprio TruthID já aplica no
    // handler HTTP (ver sign_request.rs, SIGN_REQUEST_TIMEOUT) — este client
    // só não pode expirar antes dele.
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(310))
        .build()?;
    let url = format!("http://127.0.0.1:{port}/truthid/v1/sign-request");
    let result: TruthIdWireResult = client
        .post(&url)
        .json(&serde_json::json!({
            "appName": APP_NAME,
            "dest": TEST_DEST_ADDRESS,
            "value": "0",
            "callData": "0x",
            "functionSignature": TEST_FUNCTION_SIGNATURE,
        }))
        .send()
        .await?
        .json()
        .await?;

    Ok(result.into())
}

/// Assinatura da função pro campo `functionSignature` do `/sign-request` —
/// precisa bater exatamente com `updateRecord(string calldata cid, bytes32
/// contentHash)` do contrato, senão o TruthID mostra "não verificado" na tela
/// de aprovação (mesmo mecanismo de checagem que `TEST_FUNCTION_SIGNATURE` já
/// exercita de propósito com um mismatch).
const UPDATE_RECORD_FUNCTION_SIGNATURE: &str = "updateRecord(string,bytes32)";

/// Fase 8.2 — escrita do CID de sync via o canal delegado do TruthID, só o
/// caso "mesma máquina" (reaproveita `discover()`, igual `send_test_sign_request`).
/// O CID/hash usados aqui ainda são inseridos manualmente na UI — gerar isso
/// de verdade (cifrar + subir pro IPFS) é a Fase 8.3/8.4, não esta fatia.
#[tauri::command]
pub async fn update_sync_record(
    cid: String,
    content_hash: String,
) -> Result<TruthIdSignResult, AppError> {
    let parsed_hash = sync_registry::parse_content_hash(&content_hash)?;
    let calldata = sync_registry::build_update_record_calldata(&cid, parsed_hash);

    let (port, _) = discover().await?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(310))
        .build()?;
    let url = format!("http://127.0.0.1:{port}/truthid/v1/sign-request");
    let result: TruthIdWireResult = client
        .post(&url)
        .json(&serde_json::json!({
            "appName": APP_NAME,
            "dest": sync_registry::SYNC_REGISTRY_ADDRESS,
            "value": "0",
            "callData": format!("0x{}", hex::encode(&calldata)),
            "functionSignature": UPDATE_RECORD_FUNCTION_SIGNATURE,
        }))
        .send()
        .await?
        .json()
        .await?;

    Ok(result.into())
}

// Formato de fio da resposta de `/truthid/v1/pin` (`PinResponse` em
// `pin.rs` do TruthID) — mesmo motivo de dois tipos separados que
// `TruthIdWireResult`/`TruthIdSignResult` já têm (campos ausentes viram
// `None` silenciosamente, então a convenção de nome de cada lado precisa
// bater com quem lê/escreve, camelCase vindo de fora vs snake_case indo
// pro Tauri/JS).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PinWireResult {
    status: String,
    cid: Option<String>,
    content_hash: Option<String>,
    providers_ok: Option<Vec<String>>,
    providers_failed: Option<Vec<String>>,
    error: Option<String>,
}

#[derive(Serialize)]
pub struct PinResult {
    status: String,
    cid: Option<String>,
    content_hash: Option<String>,
    providers_ok: Option<Vec<String>>,
    providers_failed: Option<Vec<String>>,
    error: Option<String>,
}

impl From<PinWireResult> for PinResult {
    fn from(wire: PinWireResult) -> Self {
        PinResult {
            status: wire.status,
            cid: wire.cid,
            content_hash: wire.content_hash,
            providers_ok: wire.providers_ok,
            providers_failed: wire.providers_failed,
            error: wire.error,
        }
    }
}

/// Fase 8.3 — pina os bytes reais do arquivo SQLite atual via o proxy de
/// pinning do TruthID (`/truthid/v1/pin`), que já cuida de aprovação
/// (park+approve, até 300s) e de subir pros providers configurados em
/// TruthID → Settings → Pinning. O `contentHash` devolvido já vem no mesmo
/// formato hex que `sync_registry::parse_content_hash` aceita, então o
/// resultado encaixa direto nos campos de `update_sync_record` (Fase 8.2)
/// sem conversão nenhuma.
///
/// Ler o arquivo `.db` bruto enquanto o próprio app pode ter uma conexão
/// aberta nele arrisca um snapshot inconsistente no meio de uma escrita
/// (torn read) — não há `journal_mode=WAL`/checkpoint configurado. Aceitável
/// pra uso pessoal single-user nesta fatia; pendência de robustez registrada
/// pra quando a Fase 8.5 for desenhada de verdade.
#[tauri::command]
pub async fn pin_database_snapshot() -> Result<PinResult, AppError> {
    let bytes = tokio::fs::read(db::DATABASE_FILE_PATH).await?;
    let content_base64 = STANDARD.encode(&bytes);

    let (port, _) = discover().await?;

    // Mesma margem de 310s que `send_test_sign_request`/`update_sync_record`
    // já usam sobre o timeout de park+approve do lado do TruthID (300s).
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(310))
        .build()?;
    let url = format!("http://127.0.0.1:{port}/truthid/v1/pin");
    let result: PinWireResult = client
        .post(&url)
        .json(&serde_json::json!({
            "appName": APP_NAME,
            "contentBase64": content_base64,
        }))
        .send()
        .await?
        .json()
        .await?;

    Ok(result.into())
}

/// Mesmo TTL que `qrPayload.ts::SESSION_TTL_MS` já usa pro pareamento do
/// Vault (extensão) — tempo suficiente pro usuário pegar o celular, escanear,
/// revisar e aprovar antes do QR expirar.
const CROSS_DEVICE_SESSION_TTL_MS: i64 = 3 * 60 * 1000;

/// Intervalo entre passadas de varredura LAN — o celular só começa a servir
/// depois que o usuário aprovar (e, no caso do sign-request, depois que a
/// UserOperation terminar de executar, até ~60s), então uma única passada
/// logo após mostrar o QR quase sempre vai vazia; o chamador repete até expirar.
const SWEEP_RETRY_INTERVAL: Duration = Duration::from_secs(2);

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_millis() as i64
}

fn random_session_id() -> String {
    let mut bytes = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Schema v1 do QR de `/sign-request` cross-device — precisa bater campo a
/// campo com `_validatePayload` em
/// `mobile/lib/screens/sign_request_approval_screen.dart` (TruthID).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SignRequestQrPayload {
    action: &'static str,
    v: u8,
    session_id: String,
    ephemeral_pub_key: String,
    expires_at: i64,
    app_name: &'static str,
    dest: &'static str,
    value: &'static str,
    call_data: &'static str,
    function_signature: &'static str,
}

#[derive(Serialize)]
pub struct CrossDeviceSession {
    session_id: String,
    ephemeral_priv_key_hex: String,
    expires_at_ms: i64,
    qr_payload_json: String,
}

/// Gera uma nova sessão cross-device de `/sign-request`: par efêmero (ECIES),
/// `sessionId` aleatório e o JSON do QR pra o frontend renderizar. Não fala
/// com a rede — só monta o convite. Quem varre a LAN esperando a resposta do
/// celular é `await_cross_device_sign_request_response`, chamado em seguida
/// pelo frontend assim que o QR aparece na tela. Reusa as mesmas constantes
/// `TEST_DEST_ADDRESS`/`TEST_FUNCTION_SIGNATURE` da PoC loopback
/// (`send_test_sign_request`) — mesma transferência de valor zero pro
/// endereço de burn, sem efeito econômico, mesma decisão da Sessão 103.
#[tauri::command]
pub fn create_cross_device_sign_request() -> Result<CrossDeviceSession, AppError> {
    let session_id = random_session_id();
    let (ephemeral_priv_key_hex, ephemeral_pub_key_hex) = ecies::generate_ephemeral_keypair();
    let expires_at_ms = now_ms() + CROSS_DEVICE_SESSION_TTL_MS;

    let payload = SignRequestQrPayload {
        action: "truthid-sign-request",
        v: 1,
        session_id: session_id.clone(),
        ephemeral_pub_key: ephemeral_pub_key_hex,
        expires_at: expires_at_ms,
        app_name: APP_NAME,
        dest: TEST_DEST_ADDRESS,
        value: "0",
        call_data: "0x",
        function_signature: TEST_FUNCTION_SIGNATURE,
    };
    let qr_payload_json =
        serde_json::to_string(&payload).map_err(|e| AppError::TruthId(e.to_string()))?;

    Ok(CrossDeviceSession {
        session_id,
        ephemeral_priv_key_hex,
        expires_at_ms,
        qr_payload_json,
    })
}

/// Intervalo entre tentativas de dead-drop — bem mais espaçado que a LAN
/// (`SWEEP_RETRY_INTERVAL`, 2s): a propagação de IPNS leva até ~1-2min, e
/// bater num gateway público a cada 2s seria agressivo demais. Mesma ordem
/// de grandeza do `chrome.alarms` da extensão (1/min), um pouco mais
/// frequente porque a sessão inteira aqui dura só 3min, não indefinidamente.
const DEAD_DROP_RETRY_INTERVAL: Duration = Duration::from_secs(20);

fn decrypt_and_parse_result(
    blob: &[u8],
    ephemeral_priv_key_hex: &str,
) -> Result<TruthIdSignResult, AppError> {
    let plaintext = ecies::decrypt(blob, ephemeral_priv_key_hex).map_err(AppError::TruthId)?;
    let wire: TruthIdWireResult =
        serde_json::from_slice(&plaintext).map_err(|e| AppError::TruthId(e.to_string()))?;
    Ok(wire.into())
}

/// Varre a LAN repetidamente (portas `lan_sweep::CANDIDATE_PORTS`, mesmo
/// bloco que `RemoteSignerLanServer` do Mobile usa) e, em paralelo com
/// cadência bem mais espaçada, tenta o dead-drop IPFS/IPNS
/// (`dead_drop::try_fetch_dead_drop`) — os dois transportes que o Mobile já
/// tenta em paralelo ao entregar a resposta (`sign_request_approval_screen.dart`).
/// O primeiro que achar um blob decide; nenhum dos dois é obrigatório (o
/// Mobile publica o dead-drop best-effort, sem provider Kubo configurado ele
/// nem tenta). Decifra o blob ECIES com a chave efêmera privada gerada por
/// `create_cross_device_sign_request` e decodifica o mesmo formato de
/// `TruthIdSignResult` que o canal loopback já usa — os dois transportes
/// carregam exatamente o mesmo blob ECIES, sem envelope extra.
#[tauri::command]
pub async fn await_cross_device_sign_request_response(
    session_id: String,
    ephemeral_priv_key_hex: String,
    expires_at_ms: i64,
) -> Result<TruthIdSignResult, AppError> {
    let client = reqwest::Client::new();
    let mut next_dead_drop_attempt_ms = now_ms();

    loop {
        if let Some(blob) = lan_sweep::sweep_once(&session_id, &client).await {
            return decrypt_and_parse_result(&blob, &ephemeral_priv_key_hex);
        }

        if now_ms() >= next_dead_drop_attempt_ms {
            if let Some(blob) = dead_drop::try_fetch_dead_drop(&session_id, &client).await {
                return decrypt_and_parse_result(&blob, &ephemeral_priv_key_hex);
            }
            next_dead_drop_attempt_ms = now_ms() + DEAD_DROP_RETRY_INTERVAL.as_millis() as i64;
        }

        if now_ms() >= expires_at_ms {
            return Err(AppError::TruthId(
                "timed out waiting for the phone to respond".to_string(),
            ));
        }

        tokio::time::sleep(SWEEP_RETRY_INTERVAL).await;
    }
}
