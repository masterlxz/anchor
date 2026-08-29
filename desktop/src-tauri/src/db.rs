use std::path::PathBuf;

use sea_orm::{Database, DatabaseConnection, DbErr};
use tauri::Manager;

// Dev-only path: caminho fixo via bind mount declarado em docker-compose.yml
// (`../data:/data`). Até a Fase 14.5, esse arquivo morava em
// `/data-collector/anchor.db` — Rust e o coletor Python compartilhavam o
// mesmo container, então só precisavam apontar pro mesmo arquivo (decisão
// original, ver `project/ARCHITECTURE.md`). Com `data-collector/` apagado
// por completo (Fase 14.5 — todo o fetch+write já mora em `finance_api/`,
// consumindo a Finance API do easybusiness via HTTP), o motivo de
// compartilhar container desapareceu; o banco ganhou uma pasta própria.
const DEV_DATABASE_FILE_PATH: &str = "/data/anchor.db";

// Fase 11.3 — em build de release (empacotado), não existe bind mount nem
// caminho fixo: o banco vai pro `app_data_dir()` do SO, mesmo mecanismo já
// usado por `commands::thesis`/`commands::property` pros anexos. Em dev
// (`cfg!(debug_assertions)`), continua o caminho hardcoded de sempre.
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
