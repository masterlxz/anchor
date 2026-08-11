use sea_orm::{DatabaseConnection, EntityTrait, QueryOrder, QuerySelect};
use serde::{Deserialize, Serialize};

use crate::commands::api_key::read_api_key_secret;
use crate::domain::chat_provider::Provider;
use crate::entity::{alert_event, valuation};
use crate::error::AppError;

// Hand-written, not generated from the domain modules — this is a compact
// briefing for the model, not a spec. Full formulas/guards live in
// PROJECT_STATE.md (Fase 3) if this ever needs updating alongside the models.
const SYSTEM_REPERTOIRE: &str = "\
Você é o assistente de IA embutido no Anchor, um app de desktop pessoal pra acompanhar teses de investimento em ações da B3 e em cripto (Ethereum). Você tem acesso só de leitura ao banco do usuário (nunca cria, edita ou apaga nada) — os dados relevantes (valuations salvas e eventos de alerta recentes) vêm anexados depois deste texto.

Regra geral de todo modelo de ação: margem_seguranca = (preco_justo - preco_atual) / preco_justo; veredito é CHEAP se margem_seguranca > 0, senão EXPENSIVE.

Os 8 modelos de valuation de ação disponíveis (campo `model` na valuation salva):
1. dcf (DCF/FCFF) — empresas 'normais' (varejo, indústria, tech, utilities), não serve pra banco/incorporadora. FCFF = EBIT×(1-IR) + D&A - Capex - ΔNWC; desconta a WACC, cresce a taxa g na perpetuidade.
2. gordon (Gordon/DDM) — boa pagadora de dividendo com crescimento previsível. Preco_Justo = D0×(1+g) / (Ke-g).
3. bazin — 'vaca leiteira' (bancão, elétrica, saneamento), foco em yield. Preco_Teto = Dividendo_Médio_5a / Yield_Desejado.
4. graham (Graham Number) — filtro rápido de margem de segurança, qualquer empresa com lucro e PL positivos. Graham_Number = RAIZ(22.5 × LPA × VPA).
5. banks (P/B via ROE-Gordon) — instituições financeiras, onde dívida é matéria-prima do negócio, não alavancagem a evitar. P/B_Justo = (ROE - g_sustentável) / (Ke - g_sustentável), g_sustentável = ROE×(1-Payout).
6. rnav — construtoras/incorporadoras, o 'estoque' é imóvel. RNAV/Ação = (Landbank + Estoque + Caixa_Líquido) / Nº_Ações.
7. projected_ceiling (Preço Teto Projetivo) — como o Bazin, mas projeta N anos de crescimento do dividendo e traz a valor presente.
8. rim (RIM — Lucro Residual) — generaliza o banks pra bancos/financeiras: projeta o ROE convergindo (fade linear) do valor atual até o próprio Ke ao longo de N anos, desconta o lucro residual [(ROE_t - Ke) × VPA_(t-1)] ano a ano a Ke. Preco_Justo = VPA0 + Σ VP(LucroResidual_t). Sem valor terminal a somar (em t=N o ROE já é Ke, lucro residual dali em diante é zero por construção). Quando ROE_atual = Ke, o preço justo bate exatamente com o valor patrimonial (mesmo caso particular do banks).

Score de cripto (Ethereum, score contínuo, atualizado ao longo do tempo, não anual como as ações): 9 indicadores on-chain/mercado (MVRV Z-Score, NVT Ratio, Puell Multiple, Emissão Líquida, Staking Yield Líquido, TVL DeFi, Endereços Ativos/Transações, Exchange Netflow, Fees de Rede vs Emissão), cada um classificado GREEN/NEUTRAL/RED contra dois limiares configuráveis. Score final = quantos indicadores estão GREEN de 9. Leitura sugerida: 7-9 verdes = tese intacta (manter/aportar); 4-6 = neutro (observar de perto); 0-3 = considerar reduzir risco/posição.

Se o usuário pedir pra você propor a criação de uma valuation, use a tool `propose_valuation`. Nunca crie nada sozinho — a tool só monta uma prévia, o usuário precisa aprovar manualmente na interface antes de qualquer escrita no banco. O campo `inputs` deve ter exatamente estas chaves, dependendo do `model` escolhido:
- dcf: ebit, tax_rate, depreciation_amortization, capex, nwc_change, total_debt, cash, shares_outstanding, beta, risk_free_rate, market_risk_premium, kd, perpetuity_growth (todos number)
- gordon: current_dividend, expected_growth, ke (todos number)
- bazin: average_dividend, desired_yield (todos number)
- graham: eps, book_value_per_share (todos number)
- banks: book_value_per_share, roe, payout, ke (todos number)
- rnav: landbank, inventory_at_market_value, net_cash, shares_outstanding (todos number)
- projected_ceiling: current_dividend, expected_growth, projection_years (inteiro), desired_yield, ke (os demais number)
- rim: book_value_per_share, roe_current, payout, ke (todos number), fade_years (inteiro)

Responda de forma direta e concisa, sempre baseado nos dados reais fornecidos abaixo. Se a pergunta não puder ser respondida com os dados disponíveis, diga isso claramente em vez de inventar números.\
";

const RECENT_ALERT_EVENTS_LIMIT: u64 = 20;

fn format_context(valuations: &[valuation::Model], events: &[alert_event::Model]) -> String {
    let valuations_block = if valuations.is_empty() {
        "Nenhuma valuation salva ainda.".to_string()
    } else {
        valuations
            .iter()
            .map(|v| {
                format!(
                    "- {} ({}, ano-ref {}, atualizado em {}): preço atual R$ {:.2}, preço justo {}, margem de segurança {}, veredito {}",
                    v.ticker,
                    v.model,
                    v.reference_year,
                    v.updated_at,
                    v.current_price,
                    v.fair_price
                        .map(|p| format!("R$ {p:.2}"))
                        .unwrap_or_else(|| "N/D".to_string()),
                    v.safety_margin
                        .map(|m| format!("{:.1}%", m * 100.0))
                        .unwrap_or_else(|| "N/D".to_string()),
                    v.verdict.as_deref().unwrap_or("N/D"),
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let events_block = if events.is_empty() {
        "Nenhum evento de alerta registrado ainda.".to_string()
    } else {
        events
            .iter()
            .map(|e| {
                format!(
                    "- [{}] {} (disparado: {})",
                    e.created_at,
                    e.message,
                    if e.is_triggered { "sim" } else { "não" },
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "{SYSTEM_REPERTOIRE}\n\n## Valuations salvas\n{valuations_block}\n\n## Eventos de alerta recentes (até {RECENT_ALERT_EVENTS_LIMIT})\n{events_block}"
    )
}

// Fase 7.10.3 — uso real de tokens devolvido por toda resposta bem-sucedida
// dos 3 provedores, lido em vez de estimado. Circula só dentro do backend;
// o frontend recebe os números já gravados na `ai_message`.
pub struct TokenUsage {
    pub input_tokens: i32,
    pub output_tokens: i32,
}

// Fase 7.10.4 — the single tool this app declares to a provider, when
// tool-calling is enabled at all (`generate_reply`'s `tools_enabled`). Only
// one tool exists so a plain `bool` flag is more honest than a `Vec` of
// tool definitions that would only ever hold zero or one entry.
const PROPOSE_VALUATION_TOOL_NAME: &str = "propose_valuation";
const PROPOSE_VALUATION_TOOL_DESCRIPTION: &str = "\
Propõe a criação de uma nova valuation salva. NÃO cria nada no banco — só \
monta uma prévia que o usuário precisa aprovar manualmente na interface. Use \
isso quando o usuário pedir explicitamente pra calcular/salvar/criar uma \
valuation com dados que ele forneceu (ou que já estão nos dados anexados). \
Os campos exatos esperados em `inputs` dependem do `model` escolhido — \
consulte a lista de modelos e seus inputs no texto de instruções do sistema.";

fn propose_valuation_tool_parameters() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "model": {
                "type": "string",
                "enum": ["dcf", "gordon", "bazin", "graham", "banks", "rnav", "projected_ceiling", "rim"]
            },
            "ticker": { "type": "string" },
            "reference_year": { "type": "integer" },
            "current_price": { "type": "number" },
            "inputs": {
                "type": "object",
                "description": "Campos específicos do modelo escolhido — ver instruções do sistema."
            }
        },
        "required": ["model", "ticker", "reference_year", "current_price", "inputs"]
    })
}

// What the model actually produced this turn. This app never round-trips a
// tool result back to the model (deliberate simplicity choice — see
// PROJECT_STATE.md Fase 7.10.4), so there's no "pending tool call" state to
// track here, just which of these two things happened.
#[derive(Debug)]
pub enum AiOutcome {
    Text(String),
    ToolCall {
        model: String,
        ticker: String,
        reference_year: i32,
        current_price: f64,
        inputs: serde_json::Value,
    },
}

// Shared by all 3 providers' response parsing: given the raw `args`/`input`
// object a provider handed back for a `propose_valuation` call, pull out
// the typed fields `AiOutcome::ToolCall` needs. A malformed/missing field
// surfaces as a normal `AppError`, same as any other bad input this app
// sees — not a panic.
fn parse_propose_valuation_args(args: serde_json::Value) -> Result<AiOutcome, AppError> {
    let invalid = || AppError::InvalidInput("propose_valuation call missing required fields".to_string());
    let model = args.get("model").and_then(|v| v.as_str()).ok_or_else(invalid)?.to_string();
    let ticker = args.get("ticker").and_then(|v| v.as_str()).ok_or_else(invalid)?.to_string();
    let reference_year = args.get("reference_year").and_then(|v| v.as_i64()).ok_or_else(invalid)? as i32;
    let current_price = args.get("current_price").and_then(|v| v.as_f64()).ok_or_else(invalid)?;
    let inputs = args.get("inputs").cloned().ok_or_else(invalid)?;
    Ok(AiOutcome::ToolCall { model, ticker, reference_year, current_price, inputs })
}

async fn build_system_instruction(db: &DatabaseConnection) -> Result<String, AppError> {
    let valuations = valuation::Entity::find()
        .order_by_desc(valuation::Column::UpdatedAt)
        .all(db)
        .await?;

    let events = alert_event::Entity::find()
        .order_by_desc(alert_event::Column::CreatedAt)
        .limit(RECENT_ALERT_EVENTS_LIMIT)
        .all(db)
        .await?;

    Ok(format_context(&valuations, &events))
}

// Mirrors the Gemini REST API shape 1:1 (contents/parts/role) so the same
// struct can grow into full chat history in Fase 7.4 without changing shape.
#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiContent {
    pub role: String,
    pub parts: Vec<GeminiPart>,
}

// `text` is optional (not the required field it used to be) because a
// function-call response part carries only `functionCall`, no `text` at
// all — Fase 7.10.4. Outgoing request parts (built by this app) always set
// `text` and leave `function_call` `None`; only inbound response parsing
// can produce the other shape.
#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiPart {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub text: Option<String>,
    #[serde(rename = "functionCall", skip_serializing_if = "Option::is_none", default)]
    pub function_call: Option<GeminiFunctionCall>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiFunctionCall {
    pub name: String,
    pub args: serde_json::Value,
}

#[derive(Serialize)]
struct GeminiRequestBody<'a> {
    system_instruction: &'a GeminiSystemInstruction,
    contents: &'a [GeminiContent],
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<[GeminiTools; 1]>,
}

#[derive(Serialize)]
struct GeminiTools {
    #[serde(rename = "functionDeclarations")]
    function_declarations: [GeminiFunctionDeclaration; 1],
}

#[derive(Serialize)]
struct GeminiFunctionDeclaration {
    name: &'static str,
    description: &'static str,
    parameters: serde_json::Value,
}

#[derive(Serialize)]
struct GeminiSystemInstruction {
    parts: Vec<GeminiPart>,
}

#[derive(Deserialize)]
struct GeminiResponseBody {
    candidates: Vec<GeminiCandidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<GeminiUsageMetadata>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
}

// `Option` on both fields (not just the wrapper) because a response cut off
// by a safety filter can omit `candidatesTokenCount` even when
// `usageMetadata` itself is present.
#[derive(Deserialize)]
struct GeminiUsageMetadata {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: Option<i32>,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: Option<i32>,
}

impl GeminiUsageMetadata {
    fn into_token_usage(self) -> TokenUsage {
        TokenUsage {
            input_tokens: self.prompt_token_count.unwrap_or(0),
            output_tokens: self.candidates_token_count.unwrap_or(0),
        }
    }
}

async fn ask_gemini_api(
    api_key: &str,
    model: &str,
    system_instruction_text: String,
    history: Vec<GeminiContent>,
    tools_enabled: bool,
) -> Result<(AiOutcome, TokenUsage), AppError> {
    let system_instruction = GeminiSystemInstruction {
        parts: vec![GeminiPart {
            text: Some(system_instruction_text),
            function_call: None,
        }],
    };

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={api_key}"
    );
    let tools = tools_enabled.then(|| {
        [GeminiTools {
            function_declarations: [GeminiFunctionDeclaration {
                name: PROPOSE_VALUATION_TOOL_NAME,
                description: PROPOSE_VALUATION_TOOL_DESCRIPTION,
                parameters: propose_valuation_tool_parameters(),
            }],
        }]
    });
    let body = GeminiRequestBody {
        system_instruction: &system_instruction,
        contents: &history,
        tools,
    };

    let client = reqwest::Client::new();
    let response = client.post(&url).json(&body).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().await.unwrap_or_default();
        return Err(AppError::GeminiApi(format!("{status}: {error_body}")));
    }

    let parsed: GeminiResponseBody = response.json().await?;
    let usage = parsed
        .usage_metadata
        .map(GeminiUsageMetadata::into_token_usage)
        .unwrap_or(TokenUsage { input_tokens: 0, output_tokens: 0 });
    let part = parsed
        .candidates
        .into_iter()
        .next()
        .and_then(|candidate| candidate.content.parts.into_iter().next())
        .ok_or_else(|| AppError::GeminiApi("empty response from Gemini".to_string()))?;
    let outcome = match part.function_call {
        Some(call) if call.name == PROPOSE_VALUATION_TOOL_NAME => parse_propose_valuation_args(call.args)?,
        _ => AiOutcome::Text(part.text.unwrap_or_default()),
    };
    Ok((outcome, usage))
}

const CLAUDE_API_VERSION: &str = "2023-06-01";
// Chat replies here are short answers about already-saved data, not long-form
// generation — same reasoning as picking a "flash-lite"-tier Gemini model.
const CLAUDE_MAX_TOKENS: u32 = 4096;

// Reused by both Claude and OpenAI — both map Gemini's "model" role to
// "assistant" and treat anything else (only ever "user" here) as "user".
fn gemini_role_to_assistant_style(role: &str) -> &'static str {
    if role == "model" {
        "assistant"
    } else {
        "user"
    }
}

fn join_parts(parts: &[GeminiPart]) -> String {
    parts
        .iter()
        .map(|part| part.text.as_deref().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("")
}

#[derive(Serialize)]
struct ClaudeMessage {
    role: &'static str,
    content: String,
}

#[derive(Serialize)]
struct ClaudeRequestBody<'a> {
    model: &'a str,
    max_tokens: u32,
    system: &'a str,
    messages: Vec<ClaudeMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<[ClaudeTool; 1]>,
}

#[derive(Serialize)]
struct ClaudeTool {
    name: &'static str,
    description: &'static str,
    // Claude's field for this is `input_schema`, not `parameters` — verified
    // against the real API docs, differs from both Gemini and OpenAI.
    input_schema: serde_json::Value,
}

#[derive(Deserialize)]
struct ClaudeResponseBody {
    content: Vec<ClaudeContentBlock>,
    usage: ClaudeUsage,
}

#[derive(Deserialize)]
struct ClaudeContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
    // Only present on a `"tool_use"` block. `input` arrives already parsed
    // (unlike OpenAI's `arguments`, which is a JSON-encoded string).
    name: Option<String>,
    input: Option<serde_json::Value>,
}

// Both fields are always present on a real 200 response — no `Option`
// needed (unlike Gemini's `usageMetadata`, which can drop the output count
// on a safety-filtered response).
#[derive(Deserialize)]
struct ClaudeUsage {
    input_tokens: i32,
    output_tokens: i32,
}

async fn ask_claude_api(
    api_key: &str,
    model: &str,
    system_instruction: String,
    history: Vec<GeminiContent>,
    tools_enabled: bool,
) -> Result<(AiOutcome, TokenUsage), AppError> {
    let messages = history
        .iter()
        .map(|content| ClaudeMessage {
            role: gemini_role_to_assistant_style(&content.role),
            content: join_parts(&content.parts),
        })
        .collect();

    let tools = tools_enabled.then(|| {
        [ClaudeTool {
            name: PROPOSE_VALUATION_TOOL_NAME,
            description: PROPOSE_VALUATION_TOOL_DESCRIPTION,
            input_schema: propose_valuation_tool_parameters(),
        }]
    });
    let body = ClaudeRequestBody {
        model,
        max_tokens: CLAUDE_MAX_TOKENS,
        system: &system_instruction,
        messages,
        tools,
    };

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", CLAUDE_API_VERSION)
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().await.unwrap_or_default();
        return Err(AppError::ClaudeApi(format!("{status}: {error_body}")));
    }

    let parsed: ClaudeResponseBody = response.json().await?;
    let usage = TokenUsage {
        input_tokens: parsed.usage.input_tokens,
        output_tokens: parsed.usage.output_tokens,
    };
    let tool_use = parsed
        .content
        .iter()
        .find(|block| block.block_type == "tool_use" && block.name.as_deref() == Some(PROPOSE_VALUATION_TOOL_NAME))
        .and_then(|block| block.input.clone());
    let outcome = match tool_use {
        Some(args) => parse_propose_valuation_args(args)?,
        None => {
            let text = parsed
                .content
                .into_iter()
                .find(|block| block.block_type == "text")
                .and_then(|block| block.text)
                .ok_or_else(|| AppError::ClaudeApi("empty response from Claude".to_string()))?;
            AiOutcome::Text(text)
        }
    };
    Ok((outcome, usage))
}

#[derive(Serialize)]
struct OpenAiMessage {
    role: &'static str,
    content: String,
}

#[derive(Serialize)]
struct OpenAiRequestBody<'a> {
    model: &'a str,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<[OpenAiTool; 1]>,
}

#[derive(Serialize)]
struct OpenAiTool {
    #[serde(rename = "type")]
    tool_type: &'static str,
    function: OpenAiFunctionDeclaration,
}

#[derive(Serialize)]
struct OpenAiFunctionDeclaration {
    name: &'static str,
    description: &'static str,
    parameters: serde_json::Value,
}

#[derive(Deserialize)]
struct OpenAiResponseBody {
    choices: Vec<OpenAiChoice>,
    usage: OpenAiUsage,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiResponseMessage,
}

#[derive(Deserialize)]
struct OpenAiResponseMessage {
    content: Option<String>,
    // Present instead of `content` when the model calls a tool. Only the
    // first call is used — this app never asks for parallel tool calls.
    tool_calls: Option<Vec<OpenAiToolCall>>,
}

#[derive(Deserialize)]
struct OpenAiToolCall {
    function: OpenAiFunctionCall,
}

#[derive(Deserialize)]
struct OpenAiFunctionCall {
    name: String,
    // JSON-encoded string, unlike Gemini's `args`/Claude's `input` which
    // arrive already parsed — verified against the real API docs, the one
    // casing/parsing trap that's easy to get backwards between providers.
    arguments: String,
}

#[derive(Deserialize)]
struct OpenAiUsage {
    prompt_tokens: i32,
    completion_tokens: i32,
}

async fn ask_openai_api(
    api_key: &str,
    model: &str,
    system_instruction: String,
    history: Vec<GeminiContent>,
    tools_enabled: bool,
) -> Result<(AiOutcome, TokenUsage), AppError> {
    // OpenAI has no separate "system" field — it's just the first message.
    let mut messages = vec![OpenAiMessage {
        role: "system",
        content: system_instruction,
    }];
    messages.extend(history.iter().map(|content| OpenAiMessage {
        role: gemini_role_to_assistant_style(&content.role),
        content: join_parts(&content.parts),
    }));

    let tools = tools_enabled.then(|| {
        [OpenAiTool {
            tool_type: "function",
            function: OpenAiFunctionDeclaration {
                name: PROPOSE_VALUATION_TOOL_NAME,
                description: PROPOSE_VALUATION_TOOL_DESCRIPTION,
                parameters: propose_valuation_tool_parameters(),
            },
        }]
    });
    let body = OpenAiRequestBody { model, messages, tools };

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().await.unwrap_or_default();
        return Err(AppError::OpenAiApi(format!("{status}: {error_body}")));
    }

    let parsed: OpenAiResponseBody = response.json().await?;
    let usage = TokenUsage {
        input_tokens: parsed.usage.prompt_tokens,
        output_tokens: parsed.usage.completion_tokens,
    };
    let message = parsed
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message)
        .ok_or_else(|| AppError::OpenAiApi("empty response from OpenAI".to_string()))?;
    let tool_call = message
        .tool_calls
        .and_then(|calls| calls.into_iter().next())
        .filter(|call| call.function.name == PROPOSE_VALUATION_TOOL_NAME);
    let outcome = match tool_call {
        Some(call) => {
            let args: serde_json::Value = serde_json::from_str(&call.function.arguments)
                .map_err(|e| AppError::OpenAiApi(format!("invalid tool call arguments: {e}")))?;
            parse_propose_valuation_args(args)?
        }
        None => AiOutcome::Text(message.content.unwrap_or_default()),
    };
    Ok((outcome, usage))
}

// Shared by `ask_ai` (floating widget) and `commands::conversation`'s saved
// conversations (Fase 7.10.2) — resolves the key, builds the read-only DB
// context, and dispatches to the right provider. Kept as a plain function
// (not a command) so both entry points stay in sync with `SYSTEM_REPERTOIRE`
// instead of duplicating this logic.
pub async fn generate_reply(
    db: &DatabaseConnection,
    key_id: i32,
    model: &str,
    history: Vec<GeminiContent>,
    tools_enabled: bool,
) -> Result<(AiOutcome, TokenUsage), AppError> {
    let (provider, api_key) = read_api_key_secret(db, key_id).await?;
    let system_instruction = build_system_instruction(db).await?;
    match provider {
        Provider::Gemini => {
            ask_gemini_api(&api_key, model, system_instruction, history, tools_enabled).await
        }
        Provider::Claude => {
            ask_claude_api(&api_key, model, system_instruction, history, tools_enabled).await
        }
        Provider::OpenAi => {
            ask_openai_api(&api_key, model, system_instruction, history, tools_enabled).await
        }
    }
}

// Sessão 60 — a variant of `generate_reply` for one-shot completions that
// don't need the valuations/alerts DB context or the `propose_valuation`
// tool (e.g. the "Sobre o ativo" AI summary in Research). Takes the system
// instruction and a single user turn directly instead of building either
// internally, so callers outside this file stay decoupled from
// `SYSTEM_REPERTOIRE`/`build_system_instruction`.
pub async fn generate_completion(
    db: &DatabaseConnection,
    key_id: i32,
    model: &str,
    system_instruction: String,
    user_prompt: String,
) -> Result<(String, TokenUsage), AppError> {
    let (provider, api_key) = read_api_key_secret(db, key_id).await?;
    let history = vec![GeminiContent {
        role: "user".to_string(),
        parts: vec![GeminiPart {
            text: Some(user_prompt),
            function_call: None,
        }],
    }];
    let (outcome, usage) = match provider {
        Provider::Gemini => {
            ask_gemini_api(&api_key, model, system_instruction, history, false).await?
        }
        Provider::Claude => {
            ask_claude_api(&api_key, model, system_instruction, history, false).await?
        }
        Provider::OpenAi => {
            ask_openai_api(&api_key, model, system_instruction, history, false).await?
        }
    };
    match outcome {
        AiOutcome::Text(text) => Ok((text, usage)),
        // `tools_enabled: false` above means no tool was ever declared to
        // the provider — same reasoning as `ask_ai`'s exhaustive match.
        AiOutcome::ToolCall { .. } => Err(AppError::InvalidInput(
            "unexpected tool call with tools disabled".to_string(),
        )),
    }
}

// Provider-generic entry point for the floating chat widget. `key_id` (Fase
// 7.9.2/7.9.3 — one of possibly several named keys per provider) replaces the
// old plain `provider` string; the provider itself is now derived from the
// chosen key's row instead of being picked by the frontend directly.
#[tauri::command]
pub async fn ask_ai(
    key_id: i32,
    model: String,
    db: tauri::State<'_, DatabaseConnection>,
    history: Vec<GeminiContent>,
) -> Result<String, AppError> {
    let (outcome, _usage) = generate_reply(db.inner(), key_id, &model, history, false).await?;
    match outcome {
        AiOutcome::Text(text) => Ok(text),
        // `tools_enabled: false` means no tool was ever declared to the
        // provider, so this arm shouldn't be reachable in practice — kept
        // exhaustive (not `unreachable!()`) so a future provider quirk fails
        // soft instead of panicking the floating widget.
        AiOutcome::ToolCall { .. } => Err(AppError::InvalidInput(
            "unexpected tool call with tools disabled".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_valuation() -> valuation::Model {
        valuation::Model {
            id: 1,
            ticker: "BBAS3".to_string(),
            reference_year: 2025,
            current_price: 25.0,
            model: "bazin".to_string(),
            fair_price: Some(30.0),
            safety_margin: Some(0.1667),
            verdict: Some("CHEAP".to_string()),
            updated_at: "2026-07-12T10:00:00-03:00".to_string(),
        }
    }

    fn sample_event() -> alert_event::Model {
        alert_event::Model {
            id: 1,
            alert_rule_id: 1,
            is_triggered: true,
            message: "BBAS3 price 25.00 now crosses fair price 30.00".to_string(),
            created_at: "2026-07-12T10:05:00-03:00".to_string(),
        }
    }

    #[test]
    fn format_context_reports_empty_sections_explicitly() {
        let text = format_context(&[], &[]);
        assert!(text.contains("Nenhuma valuation salva ainda."));
        assert!(text.contains("Nenhum evento de alerta registrado ainda."));
        assert!(text.starts_with(SYSTEM_REPERTOIRE));
    }

    #[test]
    fn format_context_includes_valuation_and_event_data() {
        let text = format_context(&[sample_valuation()], &[sample_event()]);
        assert!(text.contains("BBAS3 (bazin, ano-ref 2025"));
        assert!(text.contains("preço justo R$ 30.00"));
        assert!(text.contains("margem de segurança 16.7%"));
        assert!(text.contains("veredito CHEAP"));
        assert!(text.contains("now crosses fair price 30.00"));
        assert!(text.contains("(disparado: sim)"));
    }

    #[test]
    fn format_context_falls_back_to_nd_for_missing_fields() {
        let mut incomplete = sample_valuation();
        incomplete.fair_price = None;
        incomplete.safety_margin = None;
        incomplete.verdict = None;

        let text = format_context(&[incomplete], &[]);
        assert!(text.contains("preço justo N/D"));
        assert!(text.contains("margem de segurança N/D"));
        assert!(text.contains("veredito N/D"));
    }

    #[test]
    fn gemini_response_parses_usage_metadata() {
        let body = r#"{
            "candidates": [{"content": {"role": "model", "parts": [{"text": "oi"}]}}],
            "usageMetadata": {"promptTokenCount": 120, "candidatesTokenCount": 45, "totalTokenCount": 165}
        }"#;
        let parsed: GeminiResponseBody = serde_json::from_str(body).unwrap();
        let usage = parsed.usage_metadata.unwrap().into_token_usage();
        assert_eq!(usage.input_tokens, 120);
        assert_eq!(usage.output_tokens, 45);
    }

    #[test]
    fn gemini_response_missing_usage_metadata_defaults_to_zero() {
        let body = r#"{
            "candidates": [{"content": {"role": "model", "parts": [{"text": "oi"}]}}]
        }"#;
        let parsed: GeminiResponseBody = serde_json::from_str(body).unwrap();
        assert!(parsed.usage_metadata.is_none());
    }

    #[test]
    fn claude_response_parses_usage() {
        let body = r#"{
            "content": [{"type": "text", "text": "oi"}],
            "usage": {"input_tokens": 80, "output_tokens": 30}
        }"#;
        let parsed: ClaudeResponseBody = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.usage.input_tokens, 80);
        assert_eq!(parsed.usage.output_tokens, 30);
    }

    #[test]
    fn openai_response_parses_usage() {
        let body = r#"{
            "choices": [{"message": {"role": "assistant", "content": "oi"}}],
            "usage": {"prompt_tokens": 60, "completion_tokens": 20, "total_tokens": 80}
        }"#;
        let parsed: OpenAiResponseBody = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.usage.prompt_tokens, 60);
        assert_eq!(parsed.usage.completion_tokens, 20);
    }

    fn sample_propose_args() -> serde_json::Value {
        serde_json::json!({
            "model": "graham",
            "ticker": "PETR4",
            "reference_year": 2025,
            "current_price": 35.5,
            "inputs": { "eps": 4.2, "book_value_per_share": 22.0 }
        })
    }

    #[test]
    fn parse_propose_valuation_args_extracts_typed_fields() {
        let outcome = parse_propose_valuation_args(sample_propose_args()).unwrap();
        match outcome {
            AiOutcome::ToolCall { model, ticker, reference_year, current_price, inputs } => {
                assert_eq!(model, "graham");
                assert_eq!(ticker, "PETR4");
                assert_eq!(reference_year, 2025);
                assert_eq!(current_price, 35.5);
                assert_eq!(inputs["eps"], 4.2);
            }
            AiOutcome::Text(_) => panic!("expected a tool call"),
        }
    }

    #[test]
    fn parse_propose_valuation_args_rejects_missing_field() {
        let mut args = sample_propose_args();
        args.as_object_mut().unwrap().remove("ticker");
        assert!(parse_propose_valuation_args(args).is_err());
    }

    #[test]
    fn gemini_function_call_response_parses_to_tool_call() {
        let body = r#"{
            "candidates": [{"content": {"role": "model", "parts": [
                {"functionCall": {"name": "propose_valuation", "args": {
                    "model": "graham", "ticker": "PETR4", "reference_year": 2025,
                    "current_price": 35.5, "inputs": {"eps": 4.2, "book_value_per_share": 22.0}
                }}}
            ]}}]
        }"#;
        let parsed: GeminiResponseBody = serde_json::from_str(body).unwrap();
        let part = parsed.candidates.into_iter().next().unwrap().content.parts.into_iter().next().unwrap();
        assert!(part.text.is_none());
        let call = part.function_call.unwrap();
        assert_eq!(call.name, PROPOSE_VALUATION_TOOL_NAME);
        let outcome = parse_propose_valuation_args(call.args).unwrap();
        assert!(matches!(outcome, AiOutcome::ToolCall { ref ticker, .. } if ticker == "PETR4"));
    }

    #[test]
    fn claude_tool_use_response_parses_to_tool_call() {
        let body = r#"{
            "content": [{"type": "tool_use", "id": "toolu_1", "name": "propose_valuation", "input": {
                "model": "graham", "ticker": "PETR4", "reference_year": 2025,
                "current_price": 35.5, "inputs": {"eps": 4.2, "book_value_per_share": 22.0}
            }}],
            "usage": {"input_tokens": 80, "output_tokens": 30}
        }"#;
        let parsed: ClaudeResponseBody = serde_json::from_str(body).unwrap();
        let block = parsed.content.into_iter().next().unwrap();
        assert_eq!(block.block_type, "tool_use");
        assert_eq!(block.name.as_deref(), Some(PROPOSE_VALUATION_TOOL_NAME));
        let outcome = parse_propose_valuation_args(block.input.unwrap()).unwrap();
        assert!(matches!(outcome, AiOutcome::ToolCall { ref ticker, .. } if ticker == "PETR4"));
    }

    #[test]
    fn openai_tool_calls_response_parses_to_tool_call() {
        let body = r#"{
            "choices": [{"message": {"role": "assistant", "content": null, "tool_calls": [
                {"id": "call_1", "type": "function", "function": {
                    "name": "propose_valuation",
                    "arguments": "{\"model\":\"graham\",\"ticker\":\"PETR4\",\"reference_year\":2025,\"current_price\":35.5,\"inputs\":{\"eps\":4.2,\"book_value_per_share\":22.0}}"
                }}
            ]}}],
            "usage": {"prompt_tokens": 60, "completion_tokens": 20, "total_tokens": 80}
        }"#;
        let parsed: OpenAiResponseBody = serde_json::from_str(body).unwrap();
        let message = parsed.choices.into_iter().next().unwrap().message;
        let call = message.tool_calls.unwrap().into_iter().next().unwrap();
        assert_eq!(call.function.name, PROPOSE_VALUATION_TOOL_NAME);
        let args: serde_json::Value = serde_json::from_str(&call.function.arguments).unwrap();
        let outcome = parse_propose_valuation_args(args).unwrap();
        assert!(matches!(outcome, AiOutcome::ToolCall { ref ticker, .. } if ticker == "PETR4"));
    }
}
