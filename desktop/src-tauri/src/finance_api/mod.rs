// Fase 14.2 — infraestrutura de acesso à Finance API (easybusiness), que vai
// substituir o `data-collector/` local por completo até o fim da Fase 14.
// `sidecar` cuida do ciclo de vida do processo (release) / endpoint fixo
// (dev); `client` é o HTTP client tipado consumido por cima dele. Nenhum
// `#[tauri::command]` chama isso ainda — a Fase 14.4 é quem porta a lógica
// de fetch+write do coletor Python pra cima deste client.
pub mod client;
pub mod sidecar;

pub use sidecar::FinanceApiHandle;
