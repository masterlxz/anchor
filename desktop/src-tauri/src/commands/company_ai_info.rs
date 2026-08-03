use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, Unchanged};
use serde::Deserialize;

use crate::commands::chat::generate_completion;
use crate::entity::{ai_api_key, company_ai_info};
use crate::error::AppError;

// Sessão 60 — "Sobre o ativo" na tela de Research: um resumo em texto
// corrido gerado por IA, cacheado por `(ticker, asset_class)` pra não gastar
// tokens de novo a cada visita. `asset_class` entra no enquadramento porque
// "empresa" não faz sentido pra fundo/cripto/commodity.
fn framing_for_asset_class(asset_class: &str) -> &'static str {
    match asset_class {
        "acao_br" | "acao_internacional" | "bdr" => {
            "O ativo é uma empresa listada em bolsa (ação ou recibo representando uma). Fale sobre o que a \
            empresa faz, seu modelo de negócio/receita, o setor em que atua, um resumo da história/fundação, \
            e pontos relevantes de contexto competitivo."
        }
        "fii" | "etf_br" => {
            "O ativo é um fundo negociado em bolsa (fundo imobiliário ou fundo de índice). Fale sobre a \
            estratégia do fundo, o tipo (ex.: tijolo, papel, fundo de fundos, índice/setor específico), o que \
            ele busca entregar ao cotista, e o gestor/administrador quando relevante."
        }
        "cripto" => {
            "O ativo é uma criptomoeda/protocolo. Fale sobre o que a rede/protocolo faz, seu caso de uso \
            principal, a tecnologia por trás, e um resumo da história do projeto."
        }
        "metal" => {
            "O ativo é uma commodity (metal precioso). Fale sobre os principais usos (industrial, joalheria, \
            reserva de valor), como costuma se comportar como classe de ativo, e contexto histórico relevante."
        }
        _ => {
            "Fale sobre o que esse ativo é, seu contexto e histórico relevante."
        }
    }
}

const SYSTEM_INSTRUCTION: &str = "\
Você é o assistente de IA embutido no Practice Valuation, um app de desktop pessoal de acompanhamento de \
investimentos. Sua tarefa aqui é escrever um resumo qualitativo curto sobre um ativo específico, pra seção \
\"Sobre o ativo\" da tela de análise. Não invente números específicos (preço, cotação, indicadores financeiros \
exatos) — esses já aparecem em outras partes da tela, alimentados por fontes de dados próprias. Se não tiver \
certeza sobre desenvolvimentos muito recentes, diga isso em vez de inventar.\n\
\n\
Formate a resposta em Markdown simples pra ficar fácil de escanear visualmente — o formato importa mais que a \
prosa corrida:\n\
- Divida o conteúdo em 2 a 4 seções curtas, cada uma com um subtítulo de nível 3 (### Subtítulo) — escolha os \
subtítulos que fizerem sentido pro tipo de ativo (ex.: visão geral, modelo de negócio/estratégia, histórico, \
panorama competitivo/contexto de mercado).\n\
- Dentro de cada seção, o formato padrão é: no máximo 1 frase de abertura (pode omitir se não agregar nada), \
seguida de uma lista com `-` de 2 a 4 itens curtos — cada item com 1 frase só. NÃO escreva parágrafos de várias \
frases encadeadas: se uma ideia não cabe numa frase curta, quebre em mais itens de lista em vez de alongar a \
frase ou emendar outra.\n\
- Destaque em **negrito** só os 2 ou 3 termos/nomes mais importantes de todo o texto, não frase inteira.\n\
- Não use tabelas, blocos de código nem títulos de nível 1 ou 2. Responda só com o markdown do resumo, sem \
saudação e sem repetir o nome do ativo como título.";

#[tauri::command]
pub async fn get_company_ai_info(
    db: tauri::State<'_, DatabaseConnection>,
    ticker: String,
    asset_class: String,
) -> Result<Option<company_ai_info::Model>, AppError> {
    let ticker = ticker.trim().to_uppercase();
    Ok(company_ai_info::Entity::find()
        .filter(company_ai_info::Column::Ticker.eq(ticker))
        .filter(company_ai_info::Column::AssetClass.eq(asset_class))
        .one(db.inner())
        .await?)
}

#[derive(Deserialize)]
pub struct GenerateCompanyAiInfoRequest {
    pub key_id: i32,
    pub model: String,
    pub ticker: String,
    pub asset_class: String,
    pub reason: Option<String>,
}

#[tauri::command]
pub async fn generate_company_ai_info(
    db: tauri::State<'_, DatabaseConnection>,
    request: GenerateCompanyAiInfoRequest,
) -> Result<company_ai_info::Model, AppError> {
    let ticker = request.ticker.trim().to_uppercase();
    let db = db.inner();

    let key_row = ai_api_key::Entity::find_by_id(request.key_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("api key {}", request.key_id)))?;

    let existing = company_ai_info::Entity::find()
        .filter(company_ai_info::Column::Ticker.eq(ticker.clone()))
        .filter(company_ai_info::Column::AssetClass.eq(request.asset_class.clone()))
        .one(db)
        .await?;

    let mut prompt = format!(
        "{}\n\nTicker: {ticker}",
        framing_for_asset_class(&request.asset_class)
    );
    if let Some(reason) = request.reason.as_deref().filter(|r| !r.trim().is_empty()) {
        if let Some(previous) = &existing {
            prompt.push_str(&format!(
                "\n\nO texto anterior foi considerado insatisfatório pelo usuário pelo seguinte motivo: \
                \"{reason}\". Texto anterior:\n{}\n\nEscreva uma versão nova que corrija esse problema.",
                previous.content
            ));
        } else {
            prompt.push_str(&format!("\n\nContexto adicional pedido pelo usuário: \"{reason}\"."));
        }
    }

    let (content, _usage) = generate_completion(
        db,
        request.key_id,
        &request.model,
        SYSTEM_INSTRUCTION.to_string(),
        prompt,
    )
    .await?;

    let now = chrono::Utc::now().to_rfc3339();
    let saved = if let Some(existing) = existing {
        company_ai_info::ActiveModel {
            id: Unchanged(existing.id),
            ticker: Set(ticker),
            asset_class: Set(request.asset_class),
            content: Set(content),
            provider: Set(key_row.provider),
            model: Set(request.model),
            generated_at: Set(now),
            regenerate_reason: Set(request.reason),
        }
        .update(db)
        .await?
    } else {
        company_ai_info::ActiveModel {
            ticker: Set(ticker),
            asset_class: Set(request.asset_class),
            content: Set(content),
            provider: Set(key_row.provider),
            model: Set(request.model),
            generated_at: Set(now),
            regenerate_reason: Set(request.reason),
            ..Default::default()
        }
        .insert(db)
        .await?
    };

    Ok(saved)
}
