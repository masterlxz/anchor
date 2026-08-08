use std::path::PathBuf;

use sea_orm::{Database, DatabaseConnection, DbErr};
use tauri::Manager;

// Dev-only path: Rust and the Python data-collector share this file via the
// bind mount declared in docker-compose.yml.
const DEV_DATABASE_FILE_PATH: &str = "/data-collector/anchor.db";

// Fase 11.3 — em build de release (empacotado), não existe bind mount nem
// caminho fixo: o banco vai pro `app_data_dir()` do SO, mesmo mecanismo já
// usado por `commands::thesis`/`commands::property` pros anexos. Em dev
// (`cfg!(debug_assertions)`), continua o caminho hardcoded de sempre — o
// sidecar Python recebe o mesmo caminho via `--db-path`
// (`commands::collector::run_collector`), nunca inventa o dele.
pub(crate) fn resolve_database_path(app: &tauri::AppHandle) -> PathBuf {
    if cfg!(debug_assertions) {
        return PathBuf::from(DEV_DATABASE_FILE_PATH);
    }

    let dir = app
        .path()
        .app_data_dir()
        .expect("could not resolve app data dir");
    std::fs::create_dir_all(&dir).expect("could not create app data dir");
    dir.join("anchor.db")
}

pub async fn connect(app: &tauri::AppHandle) -> Result<DatabaseConnection, DbErr> {
    let path = resolve_database_path(app);
    let url = format!("sqlite://{}?mode=rwc", path.display());
    Database::connect(url).await
}
