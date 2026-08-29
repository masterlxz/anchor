use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use rand::distributions::Alphanumeric;
use rand::Rng;
use tauri::Manager;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

use crate::error::AppError;

// Dev-only: aponta pro `docker compose up` manual do `easybusiness` (porta e
// chave batendo com o `.env.example` de lá) — nenhum processo é spawnado em
// dev, mesma convenção de `db::resolve_database_path`/`commands::collector::
// run_collector`. Alcançável de dentro do container do Anchor porque
// `docker-compose.yml` roda com `network_mode: host`.
const DEV_BASE_URL: &str = "http://localhost:8000";
const DEV_API_KEY: &str = "local-dev-key-change-me";

const HEALTHCHECK_POLL_INTERVAL: Duration = Duration::from_millis(200);
const HEALTHCHECK_TIMEOUT: Duration = Duration::from_secs(15);
const API_KEY_LENGTH: usize = 40;

/// Handle pro processo (build de release) ou endpoint fixo (dev) da Finance
/// API — vira estado gerenciado do Tauri, consumido por `finance_api::client`.
pub struct FinanceApiHandle {
    pub base_url: String,
    pub api_key: String,
    child: Mutex<Option<CommandChild>>,
}

impl FinanceApiHandle {
    /// Mata o processo sidecar, se houver um rodando — no-op em dev, onde
    /// nada foi spawnado. Chamado no `RunEvent::ExitRequested` do `lib.rs`.
    pub fn shutdown(&self) {
        if let Some(child) = self.child.lock().unwrap().take() {
            let _ = child.kill();
        }
    }

    /// Usado só pelos testes `#[ignore]` de `finance_api::client` — aponta
    /// pra uma instância já rodando (ex.: `sidecar_main.py` do easybusiness
    /// contra SQLite, ou `docker compose up` de lá), sem processo pra matar.
    #[cfg(test)]
    pub fn for_test(base_url: String, api_key: String) -> Self {
        FinanceApiHandle {
            base_url,
            api_key,
            child: Mutex::new(None),
        }
    }
}

pub fn init(app: &tauri::AppHandle) -> Result<FinanceApiHandle, AppError> {
    if cfg!(debug_assertions) {
        return Ok(FinanceApiHandle {
            base_url: DEV_BASE_URL.to_string(),
            api_key: DEV_API_KEY.to_string(),
            child: Mutex::new(None),
        });
    }

    tauri::async_runtime::block_on(spawn_release_sidecar(app))
}

async fn spawn_release_sidecar(app: &tauri::AppHandle) -> Result<FinanceApiHandle, AppError> {
    let api_key: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(API_KEY_LENGTH)
        .map(char::from)
        .collect();

    // Banco próprio, separado do `anchor.db` (`db::resolve_database_path`) —
    // schema completamente diferente (as 22 tabelas das 5 migrations do
    // easybusiness), sem relação nenhuma com o schema do Anchor.
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|err| AppError::FinanceApi(err.to_string()))?;
    std::fs::create_dir_all(&dir)?;
    let db_path: PathBuf = dir.join("finance_api.db");
    // 4 barras no total (`sqlite://` + caminho absoluto começando com `/`) —
    // formato confirmado ao vivo do lado easybusiness (Fase 1.10 de lá,
    // Sessão 89, testado contra `sqlite:////tmp/...db`).
    let database_url = format!("sqlite:///{}", db_path.display());

    let (mut rx, child) = app
        .shell()
        .sidecar("anchor-finance-api")
        .map_err(|err| AppError::FinanceApi(err.to_string()))?
        .env("API_KEYS", &api_key)
        .env("DATABASE_URL", &database_url)
        .spawn()
        .map_err(|err| AppError::FinanceApi(err.to_string()))?;

    // Lê o stdout até achar `SIDECAR_PORT=<porta>` — sinal de prontidão que
    // `api/sidecar_main.py` (easybusiness) escreve antes de subir o uvicorn,
    // depois de rodar as migrations Alembic.
    let mut port: Option<u16> = None;
    while let Some(event) = rx.recv().await {
        if let CommandEvent::Stdout(line) = event {
            let line = String::from_utf8_lossy(&line);
            if let Some(value) = line.trim().strip_prefix("SIDECAR_PORT=") {
                port = value.parse().ok();
                break;
            }
        }
    }

    let Some(port) = port else {
        let _ = child.kill();
        return Err(AppError::FinanceApi(
            "sidecar process exited before announcing SIDECAR_PORT".to_string(),
        ));
    };

    let base_url = format!("http://127.0.0.1:{port}");
    let child = wait_for_healthy(&base_url, child).await?;

    Ok(FinanceApiHandle {
        base_url,
        api_key,
        child: Mutex::new(Some(child)),
    })
}

async fn wait_for_healthy(
    base_url: &str,
    child: CommandChild,
) -> Result<CommandChild, AppError> {
    let client = reqwest::Client::new();
    let deadline = tokio::time::Instant::now() + HEALTHCHECK_TIMEOUT;

    loop {
        if let Ok(response) = client.get(format!("{base_url}/healthz")).send().await {
            if response.status().is_success() {
                return Ok(child);
            }
        }

        if tokio::time::Instant::now() >= deadline {
            let _ = child.kill();
            return Err(AppError::FinanceApi(
                "sidecar did not become healthy in time".to_string(),
            ));
        }

        tokio::time::sleep(HEALTHCHECK_POLL_INTERVAL).await;
    }
}
