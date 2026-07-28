use base64::{engine::general_purpose::STANDARD, Engine as _};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};

use crate::commands::api_key::read_api_key_secret;
use crate::domain::chat_provider::Provider;
use crate::error::AppError;

// Fase 2.4/2.5 redesenhada (Sessão 29) — extração genérica de dado
// estruturado a partir de um PDF, via os mesmos 3 provedores/chaves já
// configurados no chat (Fase 7). Diferente de `chat.rs`: aqui é sempre uma
// chamada avulda (sem histórico, sem tool-calling, sem token usage
// persistido) — um documento + uma instrução + um schema, devolvendo o JSON
// já validado direto pro chamador. Nenhum estado fica gravado no banco; quem
// chama decide o que fazer com o resultado (ex.: preencher um campo de
// formulário ainda editável).

// Bem abaixo do teto de cada provedor (Claude ~32MB, Gemini 50MB inline,
// OpenAI 32MB) — não tem por que um app pessoal de desktop precisar de PDF
// gigante, e um teto local baixo falha rápido e claro em vez de um upload
// lento seguido de erro do provedor.
const MAX_PDF_BYTES: usize = 15 * 1024 * 1024;

const CLAUDE_API_VERSION: &str = "2023-06-01";
const CLAUDE_MAX_TOKENS: u32 = 4096;

fn check_file_size(bytes: &[u8]) -> Result<(), AppError> {
    if bytes.len() > MAX_PDF_BYTES {
        return Err(AppError::InvalidInput(format!(
            "PDF too large ({} MB, max {} MB)",
            bytes.len() / (1024 * 1024),
            MAX_PDF_BYTES / (1024 * 1024)
        )));
    }
    Ok(())
}

fn parse_json_response(text: &str) -> Result<serde_json::Value, AppError> {
    serde_json::from_str(text)
        .map_err(|e| AppError::InvalidInput(format!("model returned invalid JSON: {e}")))
}

#[tauri::command]
pub async fn extract_document_data(
    db: tauri::State<'_, DatabaseConnection>,
    key_id: i32,
    model: String,
    file_path: String,
    instructions: String,
    json_schema: serde_json::Value,
) -> Result<serde_json::Value, AppError> {
    let bytes = std::fs::read(&file_path)
        .map_err(|e| AppError::InvalidInput(format!("could not read file '{file_path}': {e}")))?;
    check_file_size(&bytes)?;
    let pdf_base64 = STANDARD.encode(&bytes);

    let (provider, api_key) = read_api_key_secret(db.inner(), key_id).await?;
    match provider {
        Provider::Gemini => {
            extract_via_gemini(&api_key, &model, &pdf_base64, &instructions, &json_schema).await
        }
        Provider::Claude => {
            extract_via_claude(&api_key, &model, &pdf_base64, &instructions, &json_schema).await
        }
        Provider::OpenAi => {
            extract_via_openai(&api_key, &model, &pdf_base64, &instructions, &json_schema).await
        }
    }
}

// --- Gemini ---

#[derive(Serialize)]
struct GeminiInlineData {
    mime_type: &'static str,
    data: String,
}

#[derive(Serialize)]
struct GeminiExtractionPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inline_data: Option<GeminiInlineData>,
}

#[derive(Serialize)]
struct GeminiExtractionContent {
    role: &'static str,
    parts: Vec<GeminiExtractionPart>,
}

// `chat.rs` already sends `system_instruction`/`contents` in snake_case (not
// `systemInstruction`) and that works against the real API — Gemini's REST
// endpoint accepts the proto's original snake_case field names on input,
// even though every *response* field comes back lowerCamelCase
// (`usageMetadata`, `functionCall`). Following that same convention here for
// `generation_config`/`response_mime_type`/`response_schema` — **not
// independently confirmed by a real call in this session** (an earlier
// WebFetch against a different Gemini doc page returned a clearly wrong/
// hallucinated shape), so this is the first thing to check if Gemini
// extraction 400s.
#[derive(Serialize)]
struct GeminiGenerationConfig<'a> {
    response_mime_type: &'static str,
    response_schema: &'a serde_json::Value,
}

#[derive(Serialize)]
struct GeminiExtractionRequestBody<'a> {
    contents: Vec<GeminiExtractionContent>,
    generation_config: GeminiGenerationConfig<'a>,
}

#[derive(Deserialize)]
struct GeminiExtractionResponseBody {
    candidates: Vec<GeminiExtractionCandidate>,
}

#[derive(Deserialize)]
struct GeminiExtractionCandidate {
    content: GeminiExtractionResponseContent,
}

#[derive(Deserialize)]
struct GeminiExtractionResponseContent {
    parts: Vec<GeminiExtractionResponsePart>,
}

#[derive(Deserialize)]
struct GeminiExtractionResponsePart {
    text: Option<String>,
}

async fn extract_via_gemini(
    api_key: &str,
    model: &str,
    pdf_base64: &str,
    instructions: &str,
    json_schema: &serde_json::Value,
) -> Result<serde_json::Value, AppError> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={api_key}"
    );
    let body = GeminiExtractionRequestBody {
        contents: vec![GeminiExtractionContent {
            role: "user",
            parts: vec![
                GeminiExtractionPart {
                    text: Some(instructions.to_string()),
                    inline_data: None,
                },
                GeminiExtractionPart {
                    text: None,
                    inline_data: Some(GeminiInlineData {
                        mime_type: "application/pdf",
                        data: pdf_base64.to_string(),
                    }),
                },
            ],
        }],
        generation_config: GeminiGenerationConfig {
            response_mime_type: "application/json",
            response_schema: json_schema,
        },
    };

    let client = reqwest::Client::new();
    let response = client.post(&url).json(&body).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().await.unwrap_or_default();
        return Err(AppError::GeminiApi(format!("{status}: {error_body}")));
    }

    let parsed: GeminiExtractionResponseBody = response.json().await?;
    let text = parsed
        .candidates
        .into_iter()
        .next()
        .and_then(|candidate| candidate.content.parts.into_iter().next())
        .and_then(|part| part.text)
        .ok_or_else(|| AppError::GeminiApi("empty response from Gemini".to_string()))?;
    parse_json_response(&text)
}

// --- Claude ---

#[derive(Serialize)]
struct ClaudeDocumentSource<'a> {
    #[serde(rename = "type")]
    source_type: &'static str,
    media_type: &'static str,
    data: &'a str,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum ClaudeExtractionContentBlock<'a> {
    #[serde(rename = "document")]
    Document { source: ClaudeDocumentSource<'a> },
    #[serde(rename = "text")]
    Text { text: &'a str },
}

#[derive(Serialize)]
struct ClaudeExtractionMessage<'a> {
    role: &'static str,
    content: Vec<ClaudeExtractionContentBlock<'a>>,
}

#[derive(Serialize)]
struct ClaudeOutputFormat<'a> {
    #[serde(rename = "type")]
    format_type: &'static str,
    schema: &'a serde_json::Value,
}

#[derive(Serialize)]
struct ClaudeOutputConfig<'a> {
    format: ClaudeOutputFormat<'a>,
}

#[derive(Serialize)]
struct ClaudeExtractionRequestBody<'a> {
    model: &'a str,
    max_tokens: u32,
    messages: Vec<ClaudeExtractionMessage<'a>>,
    output_config: ClaudeOutputConfig<'a>,
}

#[derive(Deserialize)]
struct ClaudeExtractionResponseBody {
    content: Vec<ClaudeExtractionResponseBlock>,
}

#[derive(Deserialize)]
struct ClaudeExtractionResponseBlock {
    text: Option<String>,
}

async fn extract_via_claude(
    api_key: &str,
    model: &str,
    pdf_base64: &str,
    instructions: &str,
    json_schema: &serde_json::Value,
) -> Result<serde_json::Value, AppError> {
    let body = ClaudeExtractionRequestBody {
        model,
        max_tokens: CLAUDE_MAX_TOKENS,
        messages: vec![ClaudeExtractionMessage {
            role: "user",
            content: vec![
                ClaudeExtractionContentBlock::Document {
                    source: ClaudeDocumentSource {
                        source_type: "base64",
                        media_type: "application/pdf",
                        data: pdf_base64,
                    },
                },
                ClaudeExtractionContentBlock::Text { text: instructions },
            ],
        }],
        output_config: ClaudeOutputConfig {
            format: ClaudeOutputFormat {
                format_type: "json_schema",
                schema: json_schema,
            },
        },
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

    let parsed: ClaudeExtractionResponseBody = response.json().await?;
    let text = parsed
        .content
        .into_iter()
        .find_map(|block| block.text)
        .ok_or_else(|| AppError::ClaudeApi("empty response from Claude".to_string()))?;
    parse_json_response(&text)
}

// --- OpenAI ---

#[derive(Serialize)]
struct OpenAiFileData<'a> {
    file_data: String,
    filename: &'a str,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum OpenAiExtractionContentPart<'a> {
    #[serde(rename = "text")]
    Text { text: &'a str },
    #[serde(rename = "file")]
    File { file: OpenAiFileData<'a> },
}

#[derive(Serialize)]
struct OpenAiExtractionMessage<'a> {
    role: &'static str,
    content: Vec<OpenAiExtractionContentPart<'a>>,
}

#[derive(Serialize)]
struct OpenAiJsonSchemaFormat<'a> {
    name: &'static str,
    schema: &'a serde_json::Value,
    strict: bool,
}

#[derive(Serialize)]
struct OpenAiResponseFormat<'a> {
    #[serde(rename = "type")]
    format_type: &'static str,
    json_schema: OpenAiJsonSchemaFormat<'a>,
}

#[derive(Serialize)]
struct OpenAiExtractionRequestBody<'a> {
    model: &'a str,
    messages: Vec<OpenAiExtractionMessage<'a>>,
    response_format: OpenAiResponseFormat<'a>,
}

#[derive(Deserialize)]
struct OpenAiExtractionResponseBody {
    choices: Vec<OpenAiExtractionChoice>,
}

#[derive(Deserialize)]
struct OpenAiExtractionChoice {
    message: OpenAiExtractionResponseMessage,
}

#[derive(Deserialize)]
struct OpenAiExtractionResponseMessage {
    content: Option<String>,
}

async fn extract_via_openai(
    api_key: &str,
    model: &str,
    pdf_base64: &str,
    instructions: &str,
    json_schema: &serde_json::Value,
) -> Result<serde_json::Value, AppError> {
    // OpenAI's `strict: true` structured outputs require the caller's schema
    // to already set `additionalProperties: false` — enforced by whoever
    // builds `json_schema` (the frontend), not here.
    let file_data = format!("data:application/pdf;base64,{pdf_base64}");
    let body = OpenAiExtractionRequestBody {
        model,
        messages: vec![OpenAiExtractionMessage {
            role: "user",
            content: vec![
                OpenAiExtractionContentPart::Text { text: instructions },
                OpenAiExtractionContentPart::File {
                    file: OpenAiFileData {
                        file_data,
                        filename: "document.pdf",
                    },
                },
            ],
        }],
        response_format: OpenAiResponseFormat {
            format_type: "json_schema",
            json_schema: OpenAiJsonSchemaFormat {
                name: "extraction",
                schema: json_schema,
                strict: true,
            },
        },
    };

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

    let parsed: OpenAiExtractionResponseBody = response.json().await?;
    let text = parsed
        .choices
        .into_iter()
        .next()
        .and_then(|choice| choice.message.content)
        .ok_or_else(|| AppError::OpenAiApi("empty response from OpenAI".to_string()))?;
    parse_json_response(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_file_size_accepts_small_file() {
        assert!(check_file_size(&[0u8; 1024]).is_ok());
    }

    #[test]
    fn check_file_size_rejects_oversized_file() {
        let oversized = vec![0u8; MAX_PDF_BYTES + 1];
        assert!(check_file_size(&oversized).is_err());
    }

    #[test]
    fn parse_json_response_rejects_non_json_text() {
        assert!(parse_json_response("not json at all").is_err());
    }

    #[test]
    fn gemini_extraction_response_parses_embedded_json() {
        let body = r#"{
            "candidates": [{"content": {"parts": [
                {"text": "{\"landbank_millions\": 1234.5}"}
            ]}}]
        }"#;
        let parsed: GeminiExtractionResponseBody = serde_json::from_str(body).unwrap();
        let text = parsed
            .candidates
            .into_iter()
            .next()
            .unwrap()
            .content
            .parts
            .into_iter()
            .next()
            .unwrap()
            .text
            .unwrap();
        let value = parse_json_response(&text).unwrap();
        assert_eq!(value["landbank_millions"], 1234.5);
    }

    #[test]
    fn claude_extraction_response_parses_embedded_json() {
        let body = r#"{
            "content": [{"type": "text", "text": "{\"landbank_millions\": 987.6}"}]
        }"#;
        let parsed: ClaudeExtractionResponseBody = serde_json::from_str(body).unwrap();
        let text = parsed.content.into_iter().find_map(|b| b.text).unwrap();
        let value = parse_json_response(&text).unwrap();
        assert_eq!(value["landbank_millions"], 987.6);
    }

    #[test]
    fn openai_extraction_response_parses_embedded_json() {
        let body = r#"{
            "choices": [{"message": {"content": "{\"landbank_millions\": 555.0}"}}]
        }"#;
        let parsed: OpenAiExtractionResponseBody = serde_json::from_str(body).unwrap();
        let text = parsed.choices.into_iter().next().unwrap().message.content.unwrap();
        let value = parse_json_response(&text).unwrap();
        assert_eq!(value["landbank_millions"], 555.0);
    }
}
