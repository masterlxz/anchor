use std::collections::HashSet;
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Deserialize;

/// Mesmo bloco de portas que `mobile/lib/services/remote_signer_lan_server.dart`
/// usa pra servir o resultado de `/sign-message`/`/sign-request` cross-device —
/// precisa ser espelhado manualmente aqui, é a única forma de descoberta
/// (sem mDNS/broadcast), mesmo princípio de `CANDIDATE_PORTS` em
/// `commands/truthid.rs` (canal loopback, portas diferentes).
pub const CANDIDATE_PORTS: [u16; 5] = [48050, 48051, 48052, 48053, 48054];

const DEFAULT_CONCURRENCY: usize = 50;
const DEFAULT_TIMEOUT_MS: u64 = 800;

#[derive(Deserialize)]
struct SessionBlobResponse {
    blob: String,
}

/// Todos os hosts do /24 a que `local_ip` pertence (ex: 192.168.1.1..254) —
/// mesma simplificação de prefixo fixo que
/// `extension/src/session/lanDiscovery.ts::subnetHosts` já usa (não lê o
/// prefixo real da interface).
pub fn subnet_hosts(local_ip: &str) -> Vec<String> {
    let parts: Vec<&str> = local_ip.split('.').collect();
    if parts.len() != 4 {
        return Vec::new();
    }
    let prefix = parts[..3].join(".");
    (1..=254).map(|i| format!("{prefix}.{i}")).collect()
}

/// IPs locais (IPv4, não-loopback) — equivalente Rust ao
/// `NetworkInterface.list()` do Dart (`vault_lan_server_service.dart`) /
/// `chrome.system.network.getNetworkInterfaces()` da extensão.
pub fn get_local_ips() -> Vec<String> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|iface| !iface.is_loopback())
        .filter_map(|iface| match iface.ip() {
            std::net::IpAddr::V4(ip) => Some(ip.to_string()),
            std::net::IpAddr::V6(_) => None,
        })
        .collect()
}

async fn fetch_session_blob(
    client: &reqwest::Client,
    host: &str,
    port: u16,
    session_id: &str,
) -> Option<Vec<u8>> {
    let url = format!("http://{host}:{port}/session/{session_id}");
    let resp = client
        .get(&url)
        .timeout(Duration::from_millis(DEFAULT_TIMEOUT_MS))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: SessionBlobResponse = resp.json().await.ok()?;
    STANDARD.decode(body.blob).ok()
}

/// Todos os alvos (host, porta) do(s) /24 dos IPs locais × `CANDIDATE_PORTS`
/// — extraído pra função própria porque tanto `sweep_once` (GET) quanto
/// `sweep_push_once` (PUT) varrem exatamente a mesma lista.
fn sweep_targets() -> Vec<(String, u16)> {
    let local_ips = get_local_ips();
    let mut seen_hosts = HashSet::new();
    let mut targets: Vec<(String, u16)> = Vec::new();
    for ip in &local_ips {
        for host in subnet_hosts(ip) {
            if !seen_hosts.insert(host.clone()) {
                continue;
            }
            for port in CANDIDATE_PORTS {
                targets.push((host.clone(), port));
            }
        }
    }
    targets
}

/// Varre o(s) /24 dos IPs locais × `CANDIDATE_PORTS`, em lotes paralelos, e
/// retorna o primeiro blob encontrado (ou `None` se nada bateu nesta
/// passada) — mesmo desenho de `sweepLan` na extensão (`Promise.all`
/// batelado, sai assim que alguém responde). Quem decide repetir a
/// varredura (o celular pode ainda não ter respondido) é o chamador.
pub async fn sweep_once(session_id: &str, client: &reqwest::Client) -> Option<Vec<u8>> {
    let targets = sweep_targets();
    if targets.is_empty() {
        return None;
    }

    for batch in targets.chunks(DEFAULT_CONCURRENCY) {
        let futures = batch
            .iter()
            .map(|(host, port)| fetch_session_blob(client, host, *port, session_id));
        let results = futures::future::join_all(futures).await;
        if let Some(blob) = results.into_iter().flatten().next() {
            return Some(blob);
        }
    }

    None
}

/// Empurra `encrypted_bytes` pro celular via `PUT /session/<sessionId>/content`
/// — irmã de `fetch_session_blob`, mas na direção contrária: aqui o
/// requisitante ENVIA o conteúdo a pinar (fase 1 do `/pin` cross-device),
/// espelhando `RemoteSignerLanServer.receiveOnce` no Mobile
/// (`mobile/lib/services/remote_signer_lan_server.dart`). Corpo cru, sem
/// envelope JSON — mesmo formato que o lado que recebe já espera.
async fn push_session_content(
    client: &reqwest::Client,
    host: &str,
    port: u16,
    session_id: &str,
    encrypted_bytes: &[u8],
) -> bool {
    let url = format!("http://{host}:{port}/session/{session_id}/content");
    match client
        .put(&url)
        .body(encrypted_bytes.to_vec())
        .timeout(Duration::from_millis(DEFAULT_TIMEOUT_MS))
        .send()
        .await
    {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

/// Varre os mesmos alvos que `sweep_once`, mas empurrando conteúdo em vez de
/// buscar — retorna `true` assim que algum host aceitar o PUT. Mesmo
/// espírito de "1ª passada quase sempre vazia, o chamador repete até
/// expirar" (o celular só sobe o servidor depois de escanear o QR).
pub async fn sweep_push_once(
    session_id: &str,
    encrypted_bytes: &[u8],
    client: &reqwest::Client,
) -> bool {
    let targets = sweep_targets();
    if targets.is_empty() {
        return false;
    }

    for batch in targets.chunks(DEFAULT_CONCURRENCY) {
        let futures = batch.iter().map(|(host, port)| {
            push_session_content(client, host, *port, session_id, encrypted_bytes)
        });
        let results = futures::future::join_all(futures).await;
        if results.into_iter().any(|ok| ok) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn subnet_hosts_covers_the_fixed_slash_24() {
        let hosts = subnet_hosts("192.168.1.42");
        assert_eq!(hosts.len(), 254);
        assert_eq!(hosts[0], "192.168.1.1");
        assert_eq!(hosts[253], "192.168.1.254");
    }

    #[test]
    fn subnet_hosts_rejects_malformed_ip() {
        assert!(subnet_hosts("not-an-ip").is_empty());
    }

    // Servidor de teste real (não mock) que responde exatamente como
    // `RemoteSignerLanServer.serveOnce` no Mobile: 1 GET em
    // `/session/<sessionId>` -> `{"blob": "<base64>"}`, mesmo espírito
    // "sempre I/O real" que `remote_signer_lan_server_test.dart` já segue.
    fn spawn_test_server(session_id: &'static str, blob_b64: &'static str) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let request = String::from_utf8_lossy(&buf);
                let expected_path = format!("GET /session/{session_id} ");
                let body = format!("{{\"blob\":\"{blob_b64}\"}}");
                let response = if request.starts_with(&expected_path) {
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        body.len(),
                        body
                    )
                } else {
                    "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_string()
                };
                let _ = stream.write_all(response.as_bytes());
            }
        });
        port
    }

    #[tokio::test]
    async fn fetch_session_blob_decodes_a_real_http_response() {
        let blob_b64 = "aGVsbG8="; // "hello"
        let port = spawn_test_server("session-abc", blob_b64);
        let client = reqwest::Client::new();

        let blob = fetch_session_blob(&client, "127.0.0.1", port, "session-abc").await;

        assert_eq!(blob, Some(b"hello".to_vec()));
    }

    #[tokio::test]
    async fn fetch_session_blob_returns_none_on_wrong_session_id() {
        let port = spawn_test_server("session-abc", "aGVsbG8=");
        let client = reqwest::Client::new();

        let blob = fetch_session_blob(&client, "127.0.0.1", port, "session-xyz").await;

        assert_eq!(blob, None);
    }

    #[tokio::test]
    async fn fetch_session_blob_returns_none_when_nothing_listens() {
        let client = reqwest::Client::new();
        // Porta alta improvável de ter algo escutando, mesmo espírito de
        // "timeout/connection refused vira None, nunca lança" do lado TS.
        let blob = fetch_session_blob(&client, "127.0.0.1", 1, "session-abc").await;
        assert_eq!(blob, None);
    }

    // Servidor de teste real (não mock) que responde exatamente como
    // `RemoteSignerLanServer.receiveOnce` no Mobile: 1 PUT em
    // `/session/<sessionId>/content` -> 200 vazio, capturando o corpo
    // recebido pra comparação — mesmo espírito "sempre I/O real" de
    // `spawn_test_server` acima.
    fn spawn_test_put_server(
        session_id: &'static str,
    ) -> (u16, std::sync::mpsc::Receiver<Vec<u8>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut header_buf = Vec::new();
                let mut byte = [0u8; 1];
                // Lê até o fim dos headers (`\r\n\r\n`) um byte de cada vez —
                // simples e suficiente pra um teste com corpo pequeno.
                while !header_buf.ends_with(b"\r\n\r\n") {
                    if stream.read_exact(&mut byte).is_err() {
                        return;
                    }
                    header_buf.push(byte[0]);
                }
                let headers = String::from_utf8_lossy(&header_buf);
                let expected_path = format!("PUT /session/{session_id}/content ");

                if !headers.starts_with(&expected_path) {
                    let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
                    let _ = stream.write_all(response.as_bytes());
                    return;
                }

                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        line.to_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().to_string())
                    })
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(0);

                let mut body = vec![0u8; content_length];
                if stream.read_exact(&mut body).is_err() {
                    return;
                }

                let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";
                let _ = stream.write_all(response.as_bytes());
                let _ = tx.send(body);
            }
        });
        (port, rx)
    }

    #[tokio::test]
    async fn push_session_content_delivers_the_exact_body() {
        let (port, rx) = spawn_test_put_server("session-abc");
        let client = reqwest::Client::new();
        let payload = b"encrypted-pin-content-bytes".to_vec();

        let ok = push_session_content(&client, "127.0.0.1", port, "session-abc", &payload).await;

        assert!(ok);
        let received = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("server should have received a body");
        assert_eq!(received, payload);
    }

    #[tokio::test]
    async fn push_session_content_returns_false_on_wrong_session_id() {
        let (port, _rx) = spawn_test_put_server("session-abc");
        let client = reqwest::Client::new();

        let ok = push_session_content(&client, "127.0.0.1", port, "session-xyz", b"hello").await;

        assert!(!ok);
    }

    #[tokio::test]
    async fn push_session_content_returns_false_when_nothing_listens() {
        let client = reqwest::Client::new();
        let ok = push_session_content(&client, "127.0.0.1", 1, "session-abc", b"hello").await;
        assert!(!ok);
    }
}
