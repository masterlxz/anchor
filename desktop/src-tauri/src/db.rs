use sea_orm::{Database, DatabaseConnection, DbErr};

// Dev-only path: Rust and the Python data-collector share this file via the
// bind mount declared in docker-compose.yml. Production path (outside
// Docker) is deferred to Fase 6. Also read directly (not through this
// connection) by `commands::truthid::pin_database_snapshot` (Fase 8.3).
pub(crate) const DATABASE_FILE_PATH: &str = "/data-collector/practice_valuation.db";

pub async fn connect() -> Result<DatabaseConnection, DbErr> {
    let url = format!("sqlite://{DATABASE_FILE_PATH}?mode=rwc");
    Database::connect(url).await
}
