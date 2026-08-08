use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
    Unchanged,
};
use serde::Serialize;

use crate::domain::chat_provider::Provider;
use crate::entity::ai_api_key;
use crate::error::AppError;

pub(crate) const KEYRING_SERVICE: &str = "anchor";

// Fase 11.1 — rename pra Anchor. Chaves guardadas antes do rename ficam
// presas ao serviço antigo do keyring, invisíveis pro app assim que
// `KEYRING_SERVICE` muda de valor.
const LEGACY_RENAME_KEYRING_SERVICE: &str = "practice-valuation";

// Fase 7.9.2: a provider can now have several named keys, so the keyring
// username can't just be the provider id anymore (one entry per provider,
// no room for more) — it's keyed by the row's own id instead. Renaming a key
// only ever touches the `name` column, never this username, so renames never
// touch the keyring at all.
fn keyring_username(provider: &str, id: i32) -> String {
    format!("{provider}:{id}")
}

#[derive(Serialize)]
pub struct ApiKeySummary {
    pub id: i32,
    pub provider: String,
    pub name: String,
    pub created_at: String,
}

impl From<ai_api_key::Model> for ApiKeySummary {
    fn from(row: ai_api_key::Model) -> Self {
        ApiKeySummary {
            id: row.id,
            provider: row.provider,
            name: row.name,
            created_at: row.created_at,
        }
    }
}

#[tauri::command]
pub async fn create_api_key(
    db: tauri::State<'_, DatabaseConnection>,
    provider: String,
    name: String,
    key: String,
) -> Result<i32, AppError> {
    let provider = Provider::parse(&provider)?;

    let row = ai_api_key::ActiveModel {
        provider: Set(provider.as_str().to_string()),
        name: Set(name),
        created_at: Set(chrono::Utc::now().to_rfc3339()),
        ..Default::default()
    }
    .insert(db.inner())
    .await?;

    let entry = keyring::Entry::new(KEYRING_SERVICE, &keyring_username(provider.as_str(), row.id))?;
    entry.set_password(&key)?;

    Ok(row.id)
}

// Auto-migrates a key still stored under the pre-Fase-7.9.2 scheme (username
// = the bare provider id, e.g. "gemini") into a named "Default" row — runs
// once per provider, the first time the key list is read, so a key already
// configured before this feature shipped (e.g. Gemini, set up in Sessão 15)
// isn't silently lost. Idempotent: once migrated, the old-scheme keyring
// lookup finds nothing and this is a no-op on every later call.
async fn migrate_legacy_key_if_needed(
    db: &DatabaseConnection,
    provider: Provider,
) -> Result<(), AppError> {
    let already_has_rows = ai_api_key::Entity::find()
        .filter(ai_api_key::Column::Provider.eq(provider.as_str()))
        .one(db)
        .await?
        .is_some();
    if already_has_rows {
        return Ok(());
    }

    let legacy_entry = keyring::Entry::new(KEYRING_SERVICE, provider.as_str())?;
    let legacy_key = match legacy_entry.get_password() {
        Ok(key) => key,
        Err(keyring::Error::NoEntry) => return Ok(()),
        Err(err) => return Err(err.into()),
    };

    let row = ai_api_key::ActiveModel {
        provider: Set(provider.as_str().to_string()),
        name: Set("Default".to_string()),
        created_at: Set(chrono::Utc::now().to_rfc3339()),
        ..Default::default()
    }
    .insert(db)
    .await?;

    let new_entry = keyring::Entry::new(KEYRING_SERVICE, &keyring_username(provider.as_str(), row.id))?;
    new_entry.set_password(&legacy_key)?;
    legacy_entry.delete_credential()?;

    Ok(())
}

// Fase 11.1 — mesmo espírito de `migrate_legacy_key_if_needed`, mas
// migrando pelo eixo do nome do serviço no keyring, não do esquema de
// username (que não muda — continua `keyring_username(provider, id)`).
// Roda pra toda linha já existente em `ai_api_key`; idempotente, já que a
// segunda chamada não encontra mais nada sob o serviço antigo.
async fn migrate_legacy_service_if_needed(db: &DatabaseConnection) -> Result<(), AppError> {
    let rows = ai_api_key::Entity::find().all(db).await?;
    for row in rows {
        let username = keyring_username(&row.provider, row.id);
        let legacy_entry = keyring::Entry::new(LEGACY_RENAME_KEYRING_SERVICE, &username)?;
        let key = match legacy_entry.get_password() {
            Ok(key) => key,
            Err(keyring::Error::NoEntry) => continue,
            Err(err) => return Err(err.into()),
        };

        let new_entry = keyring::Entry::new(KEYRING_SERVICE, &username)?;
        new_entry.set_password(&key)?;
        legacy_entry.delete_credential()?;
    }

    Ok(())
}

#[tauri::command]
pub async fn list_api_keys(
    db: tauri::State<'_, DatabaseConnection>,
) -> Result<Vec<ApiKeySummary>, AppError> {
    migrate_legacy_service_if_needed(db.inner()).await?;

    for provider in [Provider::Gemini, Provider::Claude, Provider::OpenAi] {
        migrate_legacy_key_if_needed(db.inner(), provider).await?;
    }

    let rows = ai_api_key::Entity::find()
        .order_by_asc(ai_api_key::Column::CreatedAt)
        .all(db.inner())
        .await?;

    Ok(rows.into_iter().map(ApiKeySummary::from).collect())
}

#[tauri::command]
pub async fn rename_api_key(
    db: tauri::State<'_, DatabaseConnection>,
    id: i32,
    name: String,
) -> Result<(), AppError> {
    ai_api_key::Entity::find_by_id(id)
        .one(db.inner())
        .await?
        .ok_or_else(|| AppError::NotFound(format!("api key {id}")))?;

    ai_api_key::ActiveModel {
        id: Unchanged(id),
        name: Set(name),
        ..Default::default()
    }
    .update(db.inner())
    .await?;

    Ok(())
}

#[tauri::command]
pub async fn delete_api_key(db: tauri::State<'_, DatabaseConnection>, id: i32) -> Result<(), AppError> {
    let row = ai_api_key::Entity::find_by_id(id)
        .one(db.inner())
        .await?
        .ok_or_else(|| AppError::NotFound(format!("api key {id}")))?;

    let entry = keyring::Entry::new(KEYRING_SERVICE, &keyring_username(&row.provider, row.id))?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => {}
        Err(err) => return Err(err.into()),
    }

    ai_api_key::Entity::delete_by_id(id).exec(db.inner()).await?;

    Ok(())
}

// Used by `commands::chat::ask_ai` — not a Tauri command itself, since the
// secret must never cross back into the frontend.
pub async fn read_api_key_secret(
    db: &DatabaseConnection,
    key_id: i32,
) -> Result<(Provider, String), AppError> {
    let row = ai_api_key::Entity::find_by_id(key_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("api key {key_id}")))?;

    let provider = Provider::parse(&row.provider)?;
    let entry = keyring::Entry::new(KEYRING_SERVICE, &keyring_username(&row.provider, row.id))?;
    let secret = match entry.get_password() {
        Ok(key) => key,
        Err(keyring::Error::NoEntry) => return Err(AppError::MissingApiKey(key_id.to_string())),
        Err(err) => return Err(err.into()),
    };

    Ok((provider, secret))
}
