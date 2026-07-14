use crate::models::{
    CompletionRequest, CompletionResult, ModelInfo, ProviderSummary, WireProtocol,
};
use crate::security::{sanitize_error, validate_endpoint};
use async_trait::async_trait;
use futures_util::StreamExt;
use rand::Rng;
use reqwest::{Client, StatusCode};
use serde_json::{Value, json};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use thiserror::Error;
use tokio::time::{sleep, timeout};
use tokio_util::sync::CancellationToken;

pub type ChunkSink = Arc<dyn Fn(String) + Send + Sync>;

pub fn install_crypto_provider() -> Result<(), String> {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    rustls::crypto::CryptoProvider::get_default()
        .is_some()
        .then_some(())
        .ok_or_else(|| "Could not install the ring TLS crypto provider.".to_string())
}

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("request cancelled")]
    Cancelled,
    #[error("{kind} timeout after {seconds}s")]
    Timeout {
        kind: &'static str,
        seconds: u64,
        partial: bool,
    },
    #[error("provider returned HTTP {status}: {message}")]
    Http {
        status: u16,
        message: String,
        retryable: bool,
        partial: bool,
    },
    #[error("provider response could not be parsed: {0}")]
    Protocol(String),
    #[error("network error: {0}")]
    Network(String),
}

impl ProviderError {
    pub fn retryable(&self) -> bool {
        matches!(
            self,
            Self::Timeout { .. }
                | Self::Network(_)
                | Self::Http {
                    retryable: true,
                    ..
                }
        )
    }
    pub fn partial(&self) -> bool {
        matches!(
            self,
            Self::Timeout { partial: true, .. } | Self::Http { partial: true, .. }
        )
    }
    pub fn user_message(&self) -> String {
        sanitize_error(&self.to_string())
    }
}

#[async_trait]
pub trait ModelProvider: Send + Sync {
    async fn stream(
        &self,
        request: CompletionRequest,
        cancellation: CancellationToken,
        sink: ChunkSink,
    ) -> Result<CompletionResult, ProviderError>;

    async fn complete(
        &self,
        request: CompletionRequest,
        cancellation: CancellationToken,
    ) -> Result<CompletionResult, ProviderError> {
        self.stream(request, cancellation, Arc::new(|_| {})).await
    }
}

pub struct DemoProvider;

#[async_trait]
impl ModelProvider for DemoProvider {
    async fn stream(
        &self,
        request: CompletionRequest,
        cancellation: CancellationToken,
        sink: ChunkSink,
    ) -> Result<CompletionResult, ProviderError> {
        let aspect_lines: Vec<&str> = request
            .prompt
            .lines()
            .filter(|line| line.starts_with("- "))
            .take(5)
            .collect();
        let mut content = format!(
            "### Independent recommendation\n\n{}\n\n",
            "Turn the objective into a falsifiable decision and test the riskiest assumption before committing to the full solution."
        );
        for line in aspect_lines {
            content.push_str(&format!("**{}** Define a measurable threshold, an accountable owner, and a reversible failure condition.\n\n", line.trim_start_matches("- ")));
        }
        content.push_str("**Decision test:** proceed only when evidence clears the agreed thresholds; otherwise revisit the cheapest disputed assumption.");
        for word in content.split_inclusive(char::is_whitespace) {
            tokio::select! {
                _ = cancellation.cancelled() => return Err(ProviderError::Cancelled),
                _ = sleep(Duration::from_millis(12)) => sink(word.to_string()),
            }
        }
        Ok(CompletionResult {
            content,
            input_tokens: Some((request.prompt.len() / 4) as u64),
            output_tokens: Some((request.max_tokens as usize).min(180) as u64),
        })
    }
}

pub struct HttpProvider {
    provider: ProviderSummary,
    api_key: String,
    client: Client,
}

impl HttpProvider {
    pub fn new(provider: ProviderSummary, api_key: String) -> Result<Self, ProviderError> {
        install_crypto_provider().map_err(ProviderError::Protocol)?;
        validate_endpoint(&provider.base_url).map_err(ProviderError::Protocol)?;
        tracing::info!(provider_id = %provider.id, "building shared HTTP provider client");
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(provider.timeout.connect_secs))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("AgenticCouncil/0.1")
            .build()
            .map_err(|error| ProviderError::Network(error.to_string()))?;
        tracing::info!(provider_id = %provider.id, "shared HTTP provider client ready");
        Ok(Self {
            provider,
            api_key,
            client,
        })
    }

    fn request_builder(
        &self,
        request: &CompletionRequest,
    ) -> Result<reqwest::RequestBuilder, ProviderError> {
        let base = self.provider.base_url.trim_end_matches('/');
        match self.provider.protocol {
            WireProtocol::OpenAi => {
                let user_content = if request.images.is_empty() {
                    Value::String(request.prompt.clone())
                } else {
                    let mut parts = vec![json!({"type": "text", "text": request.prompt})];
                    parts.extend(request.images.iter().map(|image| json!({"type": "image_url", "image_url": {"url": format!("data:{};base64,{}", image.media_type, image.data_base64)}})));
                    Value::Array(parts)
                };
                let mut body = json!({
                    "model": request.model,
                    "messages": [{"role": "system", "content": request.system}, {"role": "user", "content": user_content}],
                    "stream": true
                });
                let model = request.model.to_ascii_lowercase();
                let openai_reasoning_model = self.provider.id == "openai"
                    && (model.starts_with("o1")
                        || model.starts_with("o3")
                        || model.starts_with("o4")
                        || model.starts_with("gpt-5"));
                let output_limit = if openai_reasoning_model {
                    "max_completion_tokens"
                } else {
                    "max_tokens"
                };
                body[output_limit] = json!(request.max_tokens);
                if !openai_reasoning_model {
                    body["temperature"] = json!(request.temperature);
                }
                if matches!(
                    self.provider.id.as_str(),
                    "openai" | "openrouter" | "groq" | "deepseek"
                ) {
                    body["stream_options"] = json!({"include_usage": true});
                }
                if matches!(self.provider.id.as_str(), "deepseek" | "zai")
                    && let Some(enabled) = request.thinking_enabled
                {
                    body["thinking"] =
                        json!({"type": if enabled { "enabled" } else { "disabled" }});
                }
                Ok(self
                    .client
                    .post(format!("{base}/chat/completions"))
                    .bearer_auth(&self.api_key)
                    .json(&body))
            }
            WireProtocol::Anthropic => {
                let mut content = vec![json!({"type": "text", "text": request.prompt})];
                content.extend(request.images.iter().map(|image| json!({"type": "image", "source": {"type": "base64", "media_type": image.media_type, "data": image.data_base64}})));
                Ok(self
                    .client
                    .post(format!("{base}/messages"))
                    .header("x-api-key", &self.api_key)
                    .header("anthropic-version", "2023-06-01")
                    .json(&json!({
                    "model": request.model,
                    "system": request.system,
                    "messages": [{"role": "user", "content": content}],
                    "max_tokens": request.max_tokens,
                    "temperature": request.temperature,
                    "stream": true
                    })))
            }
            WireProtocol::Gemini => {
                let encoded_model: String =
                    url::form_urlencoded::byte_serialize(request.model.as_bytes()).collect();
                let mut parts = vec![json!({"text": request.prompt})];
                parts.extend(request.images.iter().map(|image| json!({"inlineData": {"mimeType": image.media_type, "data": image.data_base64}})));
                Ok(self.client.post(format!("{base}/models/{encoded_model}:streamGenerateContent?alt=sse"))
                    .header("x-goog-api-key", &self.api_key)
                    .json(&json!({
                        "systemInstruction": {"parts": [{"text": request.system}]},
                        "contents": [{"role": "user", "parts": parts}],
                        "generationConfig": {"maxOutputTokens": request.max_tokens, "temperature": request.temperature}
                    })))
            }
            WireProtocol::Demo => Err(ProviderError::Protocol(
                "demo provider cannot use the HTTP transport".into(),
            )),
        }
    }

    async fn attempt(
        &self,
        request: &CompletionRequest,
        cancellation: &CancellationToken,
        sink: &ChunkSink,
    ) -> Result<CompletionResult, ProviderError> {
        let started = Instant::now();
        let send = self.request_builder(request)?.send();
        let response = tokio::select! {
            _ = cancellation.cancelled() => return Err(ProviderError::Cancelled),
            result = timeout(Duration::from_secs(self.provider.timeout.first_token_secs), send) => {
                match result {
                    Ok(Ok(response)) => response,
                    Ok(Err(error)) => return Err(ProviderError::Network(sanitize_error(&error.to_string()))),
                    Err(_) => return Err(ProviderError::Timeout { kind: "first-token", seconds: self.provider.timeout.first_token_secs, partial: false }),
                }
            }
        };
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            let message = sanitize_error(&body.chars().take(800).collect::<String>());
            return Err(ProviderError::Http {
                status: status.as_u16(),
                message,
                retryable: retryable_status(status),
                partial: false,
            });
        }
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut result = CompletionResult::default();
        let mut saw_content = false;
        let mut saw_activity = false;
        loop {
            let remaining = self
                .provider
                .timeout
                .total_secs
                .saturating_sub(started.elapsed().as_secs());
            if remaining == 0 {
                return Err(ProviderError::Timeout {
                    kind: "total request",
                    seconds: self.provider.timeout.total_secs,
                    partial: saw_content,
                });
            }
            let wait_secs = if saw_activity {
                self.provider.timeout.idle_stream_secs.min(remaining)
            } else {
                self.provider.timeout.first_token_secs.min(remaining)
            };
            let next = tokio::select! {
                _ = cancellation.cancelled() => return Err(ProviderError::Cancelled),
                value = timeout(Duration::from_secs(wait_secs), stream.next()) => match value {
                    Ok(value) => value,
                    Err(_) => return Err(ProviderError::Timeout { kind: if saw_activity { "idle stream" } else { "first-token" }, seconds: wait_secs, partial: saw_content }),
                }
            };
            let Some(item) = next else { break };
            let bytes =
                item.map_err(|error| ProviderError::Network(sanitize_error(&error.to_string())))?;
            buffer.push_str(&String::from_utf8_lossy(&bytes));
            for payload in drain_sse_payloads(&mut buffer) {
                if payload == "[DONE]" {
                    continue;
                }
                let value: Value = serde_json::from_str(&payload).map_err(|error| {
                    ProviderError::Protocol(format!("invalid SSE JSON: {error}"))
                })?;
                if let Some(message) = provider_error_message(&value) {
                    return Err(ProviderError::Protocol(message));
                }
                let parsed = parse_stream_value(&self.provider.protocol, &value);
                saw_activity |= parsed.activity;
                if let Some(delta) = parsed.delta
                    && !delta.is_empty()
                {
                    saw_content = true;
                    result.content.push_str(&delta);
                    sink(delta);
                }
                if parsed.input_tokens.is_some() {
                    result.input_tokens = parsed.input_tokens;
                }
                if parsed.output_tokens.is_some() {
                    result.output_tokens = parsed.output_tokens;
                }
            }
        }
        if !buffer.trim().is_empty() {
            let trailing = buffer.trim();
            let payloads = if trailing.starts_with('{') || trailing.starts_with('[') {
                vec![trailing.to_string()]
            } else {
                drain_sse_payloads(&mut format!("{buffer}\n\n"))
            };
            for payload in payloads {
                if payload == "[DONE]" {
                    continue;
                }
                if let Ok(value) = serde_json::from_str::<Value>(&payload) {
                    if let Some(message) = provider_error_message(&value) {
                        return Err(ProviderError::Protocol(message));
                    }
                    let parsed = parse_stream_value(&self.provider.protocol, &value);
                    saw_activity |= parsed.activity;
                    if let Some(delta) = parsed.delta
                        && !delta.is_empty()
                    {
                        result.content.push_str(&delta);
                        sink(delta);
                    }
                    result.input_tokens = parsed.input_tokens.or(result.input_tokens);
                    result.output_tokens = parsed.output_tokens.or(result.output_tokens);
                }
            }
        }
        if result.content.is_empty() {
            return Err(ProviderError::Protocol(if saw_activity {
                "provider stream ended after reasoning or control events but before final answer text".into()
            } else {
                "provider stream ended without text".into()
            }));
        }
        Ok(result)
    }
}

#[async_trait]
impl ModelProvider for HttpProvider {
    async fn stream(
        &self,
        request: CompletionRequest,
        cancellation: CancellationToken,
        sink: ChunkSink,
    ) -> Result<CompletionResult, ProviderError> {
        let mut last_error = None;
        for attempt in 1..=self.provider.timeout.max_attempts.max(1) {
            match self.attempt(&request, &cancellation, &sink).await {
                Ok(result) => return Ok(result),
                Err(ProviderError::Cancelled) => return Err(ProviderError::Cancelled),
                Err(error) => {
                    let should_retry =
                        error.retryable() && attempt < self.provider.timeout.max_attempts;
                    if !should_retry {
                        return Err(error);
                    }
                    if error.partial() {
                        tracing::warn!(provider = %self.provider.id, attempt, "partial stream will be retried; duplicate usage charges are possible");
                    }
                    last_error = Some(error);
                    let base_ms = 1000_u64.saturating_mul(1_u64 << (attempt - 1).min(4));
                    let jitter = rand::rng().random_range(0..=350_u64);
                    tokio::select! {
                        _ = cancellation.cancelled() => return Err(ProviderError::Cancelled),
                        _ = sleep(Duration::from_millis(base_ms + jitter)) => {}
                    }
                }
            }
        }
        Err(last_error.unwrap_or_else(|| {
            ProviderError::Network("request failed without a diagnostic".into())
        }))
    }
}

fn retryable_status(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::TOO_MANY_REQUESTS
            | StatusCode::INTERNAL_SERVER_ERROR
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT
    )
}

#[derive(Default)]
struct ParsedStreamValue {
    delta: Option<String>,
    activity: bool,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
}

fn parse_stream_value(protocol: &WireProtocol, value: &Value) -> ParsedStreamValue {
    match protocol {
        WireProtocol::OpenAi => {
            let delta = text_at(value, "/choices/0/delta/content")
                .or_else(|| text_at(value, "/choices/0/message/content"))
                .or_else(|| text_at(value, "/choices/0/text"))
                .or_else(|| text_at(value, "/output_text"))
                .or_else(|| text_parts(value.pointer("/output"), false));
            let reasoning = text_at(value, "/choices/0/delta/reasoning_content")
                .or_else(|| text_at(value, "/choices/0/delta/reasoning"))
                .or_else(|| text_at(value, "/choices/0/message/reasoning_content"));
            ParsedStreamValue {
                activity: delta.is_some()
                    || reasoning.is_some()
                    || value.pointer("/choices/0/finish_reason").is_some(),
                delta,
                input_tokens: value
                    .pointer("/usage/prompt_tokens")
                    .and_then(Value::as_u64)
                    .or_else(|| value.pointer("/usage/input_tokens").and_then(Value::as_u64)),
                output_tokens: value
                    .pointer("/usage/completion_tokens")
                    .and_then(Value::as_u64)
                    .or_else(|| {
                        value
                            .pointer("/usage/output_tokens")
                            .and_then(Value::as_u64)
                    }),
            }
        }
        WireProtocol::Anthropic => {
            let delta =
                text_at(value, "/delta/text").or_else(|| text_parts(value.get("content"), false));
            ParsedStreamValue {
                activity: delta.is_some()
                    || value
                        .pointer("/delta/thinking")
                        .and_then(Value::as_str)
                        .is_some()
                    || value.get("type").is_some(),
                delta,
                input_tokens: value
                    .pointer("/message/usage/input_tokens")
                    .and_then(Value::as_u64),
                output_tokens: value
                    .pointer("/usage/output_tokens")
                    .and_then(Value::as_u64)
                    .or_else(|| {
                        value
                            .pointer("/message/usage/output_tokens")
                            .and_then(Value::as_u64)
                    }),
            }
        }
        WireProtocol::Gemini => {
            let delta = text_parts(value.pointer("/candidates/0/content/parts"), true);
            ParsedStreamValue {
                activity: delta.is_some()
                    || value.pointer("/candidates/0/finishReason").is_some()
                    || value.get("usageMetadata").is_some(),
                delta,
                input_tokens: value
                    .pointer("/usageMetadata/promptTokenCount")
                    .and_then(Value::as_u64),
                output_tokens: value
                    .pointer("/usageMetadata/candidatesTokenCount")
                    .and_then(Value::as_u64),
            }
        }
        WireProtocol::Demo => ParsedStreamValue::default(),
    }
}

fn text_at(value: &Value, pointer: &str) -> Option<String> {
    let value = value.pointer(pointer)?;
    if let Some(text) = value.as_str() {
        return (!text.is_empty()).then(|| text.to_string());
    }
    text_parts(Some(value), false)
}

fn text_parts(value: Option<&Value>, skip_thought: bool) -> Option<String> {
    let items = value?.as_array()?;
    let text = items
        .iter()
        .filter(|part| {
            !skip_thought
                || !part
                    .get("thought")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
        })
        .filter_map(|part| {
            part.get("text")
                .and_then(Value::as_str)
                .or_else(|| part.pointer("/content/0/text").and_then(Value::as_str))
        })
        .collect::<String>();
    (!text.is_empty()).then_some(text)
}

fn provider_error_message(value: &Value) -> Option<String> {
    let error = value.get("error")?;
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .or_else(|| error.as_str())
        .unwrap_or("provider reported an error inside the response stream");
    Some(sanitize_error(message))
}

fn drain_sse_payloads(buffer: &mut String) -> Vec<String> {
    let normalized = buffer.replace("\r\n", "\n");
    *buffer = normalized;
    let mut payloads = vec![];
    while let Some(boundary) = buffer.find("\n\n") {
        let event = buffer[..boundary].to_string();
        buffer.drain(..boundary + 2);
        let data = event
            .lines()
            .filter_map(|line| line.strip_prefix("data:"))
            .map(str::trim_start)
            .collect::<Vec<_>>()
            .join("\n");
        if !data.is_empty() {
            payloads.push(data);
        }
    }
    payloads
}

pub async fn discover_models(
    provider: &ProviderSummary,
    api_key: &str,
) -> Result<Vec<ModelInfo>, ProviderError> {
    install_crypto_provider().map_err(ProviderError::Protocol)?;
    if !provider.supports_discovery {
        return Err(ProviderError::Protocol(format!(
            "{} does not advertise model discovery support",
            provider.name
        )));
    }
    validate_endpoint(&provider.base_url).map_err(ProviderError::Protocol)?;
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(provider.timeout.connect_secs))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("AgenticCouncil/0.1")
        .build()
        .map_err(|error| ProviderError::Network(error.to_string()))?;
    let base = provider.base_url.trim_end_matches('/');
    let request = match provider.protocol {
        WireProtocol::OpenAi => client.get(format!("{base}/models")).bearer_auth(api_key),
        WireProtocol::Anthropic => client
            .get(format!("{base}/models?limit=1000"))
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01"),
        WireProtocol::Gemini => client
            .get(format!("{base}/models?pageSize=1000"))
            .header("x-goog-api-key", api_key),
        WireProtocol::Demo => {
            return Err(ProviderError::Protocol(format!(
                "{} does not expose a supported model catalog endpoint",
                provider.name
            )));
        }
    };
    let response = timeout(
        Duration::from_secs(provider.timeout.first_token_secs),
        request.send(),
    )
    .await
    .map_err(|_| ProviderError::Timeout {
        kind: "model discovery",
        seconds: provider.timeout.first_token_secs,
        partial: false,
    })?
    .map_err(|error| ProviderError::Network(sanitize_error(&error.to_string())))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(ProviderError::Http {
            status: status.as_u16(),
            message: sanitize_error(&body.chars().take(800).collect::<String>()),
            retryable: retryable_status(status),
            partial: false,
        });
    }
    let value = response
        .json::<Value>()
        .await
        .map_err(|error| ProviderError::Protocol(format!("invalid model catalog JSON: {error}")))?;
    let mut models = parse_model_catalog(provider, &value);
    if models.is_empty() {
        return Err(ProviderError::Protocol(
            "the provider returned no compatible generation models".into(),
        ));
    }
    models.sort_by_cached_key(|model| model.name.to_ascii_lowercase());
    models.dedup_by(|left, right| left.id == right.id);
    models.truncate(2_000);
    Ok(models)
}

fn parse_model_catalog(provider: &ProviderSummary, value: &Value) -> Vec<ModelInfo> {
    let rows = match provider.protocol {
        WireProtocol::OpenAi | WireProtocol::Anthropic => {
            value.get("data").and_then(Value::as_array)
        }
        WireProtocol::Gemini => value.get("models").and_then(Value::as_array),
        WireProtocol::Demo => None,
    };
    rows.into_iter()
        .flatten()
        .filter_map(|row| match provider.protocol {
            WireProtocol::OpenAi => openai_model_info(provider, row),
            WireProtocol::Anthropic => anthropic_model_info(provider, row),
            WireProtocol::Gemini => gemini_model_info(provider, row),
            WireProtocol::Demo => None,
        })
        .collect()
}

fn anthropic_model_info(provider: &ProviderSummary, row: &Value) -> Option<ModelInfo> {
    let id = row.get("id")?.as_str()?.trim();
    if id.is_empty() {
        return None;
    }
    let lower = id.to_ascii_lowercase();
    Some(ModelInfo {
        id: id.to_string(),
        provider_id: provider.id.clone(),
        name: row
            .get("display_name")
            .and_then(Value::as_str)
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(id)
            .trim()
            .to_string(),
        context_window: 200_000,
        input_per_million: None,
        output_per_million: None,
        supports_vision: true,
        supports_documents: true,
        supports_streaming: true,
        reasoning: lower.contains("sonnet") || lower.contains("opus"),
    })
}

fn openai_model_info(provider: &ProviderSummary, row: &Value) -> Option<ModelInfo> {
    let id = row.get("id")?.as_str()?.trim();
    if id.is_empty() || !is_generation_model(id) {
        return None;
    }
    let lower = id.to_ascii_lowercase();
    let modalities = row
        .pointer("/architecture/input_modalities")
        .and_then(Value::as_array);
    let supports_vision = modalities.is_some_and(|values| {
        values
            .iter()
            .any(|value| value.as_str().is_some_and(|item| item == "image"))
    }) || ["vision", "gpt-4o", "gpt-4.1", "gpt-5"]
        .iter()
        .any(|marker| lower.contains(marker));
    Some(ModelInfo {
        id: id.to_string(),
        provider_id: provider.id.clone(),
        name: row
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(id)
            .trim()
            .to_string(),
        context_window: row
            .get("context_length")
            .and_then(Value::as_u64)
            .or_else(|| row.get("context_window").and_then(Value::as_u64))
            .or_else(|| {
                row.pointer("/top_provider/context_length")
                    .and_then(Value::as_u64)
            })
            .unwrap_or(128_000),
        input_per_million: price_per_million(row.pointer("/pricing/prompt")),
        output_per_million: price_per_million(row.pointer("/pricing/completion")),
        supports_vision,
        supports_documents: true,
        supports_streaming: true,
        reasoning: ["reason", "thinking", "deepseek-r1", "/o1", "/o3", "/o4"]
            .iter()
            .any(|marker| lower.contains(marker))
            || lower.starts_with("o1")
            || lower.starts_with("o3")
            || lower.starts_with("o4"),
    })
}

fn gemini_model_info(provider: &ProviderSummary, row: &Value) -> Option<ModelInfo> {
    let methods = row
        .get("supportedGenerationMethods")
        .and_then(Value::as_array);
    if methods.is_some_and(|values| {
        !values.iter().any(|value| {
            value
                .as_str()
                .is_some_and(|method| method.eq_ignore_ascii_case("generateContent"))
        })
    }) {
        return None;
    }
    let id = row.get("name")?.as_str()?.trim().strip_prefix("models/")?;
    if id.is_empty() {
        return None;
    }
    let lower = id.to_ascii_lowercase();
    Some(ModelInfo {
        id: id.to_string(),
        provider_id: provider.id.clone(),
        name: row
            .get("displayName")
            .and_then(Value::as_str)
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(id)
            .trim()
            .to_string(),
        context_window: row
            .get("inputTokenLimit")
            .and_then(Value::as_u64)
            .unwrap_or(1_000_000),
        input_per_million: None,
        output_per_million: None,
        supports_vision: !lower.contains("embedding"),
        supports_documents: true,
        supports_streaming: true,
        reasoning: lower.contains("thinking") || lower.contains("pro"),
    })
}

fn price_per_million(value: Option<&Value>) -> Option<f64> {
    let price = value.and_then(|value| {
        value
            .as_f64()
            .or_else(|| value.as_str().and_then(|item| item.parse::<f64>().ok()))
    })?;
    (price.is_finite() && price >= 0.0).then_some(price * 1_000_000.0)
}

fn is_generation_model(id: &str) -> bool {
    let lower = id.to_ascii_lowercase();
    ![
        "embedding",
        "whisper",
        "moderation",
        "dall-e",
        "tts-",
        "transcribe",
        "image-generation",
        "realtime-preview",
        "babbage",
        "davinci",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

pub fn make_provider(
    provider: &ProviderSummary,
    api_key: Option<String>,
) -> Result<Box<dyn ModelProvider>, ProviderError> {
    if provider.protocol == WireProtocol::Demo {
        return Ok(Box::new(DemoProvider));
    }
    let key = api_key.filter(|value| !value.is_empty()).ok_or_else(|| {
        ProviderError::Protocol(format!("No API key is stored for {}", provider.name))
    })?;
    Ok(Box::new(HttpProvider::new(provider.clone(), key)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{header, method, path},
    };

    #[test]
    fn parses_fragmented_sse_frames() {
        let mut buffer = "data: {\"choices\":[{\"delta\":{\"content\":\"Hi\"}}]}\n".to_string();
        assert!(drain_sse_payloads(&mut buffer).is_empty());
        buffer.push('\n');
        let values = drain_sse_payloads(&mut buffer);
        let json: Value = serde_json::from_str(&values[0]).unwrap();
        assert_eq!(
            parse_stream_value(&WireProtocol::OpenAi, &json)
                .delta
                .as_deref(),
            Some("Hi")
        );
    }

    #[test]
    fn recognizes_reasoning_activity_without_exposing_chain_of_thought() {
        let value = json!({"choices": [{"delta": {"reasoning_content": "hidden reasoning"}}]});
        let parsed = parse_stream_value(&WireProtocol::OpenAi, &value);
        assert!(parsed.activity);
        assert!(parsed.delta.is_none());
    }

    #[test]
    fn parses_non_streaming_openai_compatible_json() {
        let value = json!({
            "choices": [{"message": {"content": "OK"}}],
            "usage": {"prompt_tokens": 2, "completion_tokens": 1}
        });
        let parsed = parse_stream_value(&WireProtocol::OpenAi, &value);
        assert_eq!(parsed.delta.as_deref(), Some("OK"));
        assert_eq!(parsed.input_tokens, Some(2));
        assert_eq!(parsed.output_tokens, Some(1));
    }

    #[test]
    fn joins_gemini_text_parts_and_omits_thought_parts() {
        let value = json!({"candidates": [{"content": {"parts": [
            {"text": "private thought", "thought": true},
            {"text": "Public "},
            {"text": "answer"}
        ]}}]});
        let parsed = parse_stream_value(&WireProtocol::Gemini, &value);
        assert_eq!(parsed.delta.as_deref(), Some("Public answer"));
    }

    #[test]
    fn every_visible_catalog_provider_builds_its_wire_request() {
        for provider in catalog::providers()
            .into_iter()
            .filter(|provider| provider.protocol != WireProtocol::Demo)
        {
            let model = catalog::models()
                .into_iter()
                .find(|model| model.provider_id == provider.id)
                .map(|model| model.id)
                .unwrap_or_else(|| "validation-model".into());
            let client = HttpProvider::new(provider.clone(), "test-secret".into()).unwrap();
            let request = client
                .request_builder(&CompletionRequest {
                    system: "Connection check".into(),
                    prompt: "OK".into(),
                    model,
                    max_tokens: 256,
                    temperature: 0.0,
                    thinking_enabled: Some(false),
                    images: vec![],
                })
                .unwrap()
                .build()
                .unwrap();
            match provider.protocol {
                WireProtocol::OpenAi => {
                    assert!(request.url().path().ends_with("/chat/completions"));
                    assert!(request.headers().contains_key("authorization"));
                    let body: Value = serde_json::from_slice(
                        request.body().and_then(reqwest::Body::as_bytes).unwrap(),
                    )
                    .unwrap();
                    assert_eq!(body.get("stream").and_then(Value::as_bool), Some(true));
                    if matches!(provider.id.as_str(), "deepseek" | "zai") {
                        assert_eq!(
                            body.pointer("/thinking/type").and_then(Value::as_str),
                            Some("disabled")
                        );
                    }
                }
                WireProtocol::Anthropic => {
                    assert!(request.url().path().ends_with("/messages"));
                    assert!(request.headers().contains_key("x-api-key"));
                    assert!(request.headers().contains_key("anthropic-version"));
                }
                WireProtocol::Gemini => {
                    assert!(request.url().path().contains(":streamGenerateContent"));
                    assert!(request.headers().contains_key("x-goog-api-key"));
                }
                WireProtocol::Demo => unreachable!(),
            }
        }
    }

    #[test]
    fn classifies_retryable_status_codes() {
        assert!(retryable_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(!retryable_status(StatusCode::UNAUTHORIZED));
    }

    #[test]
    fn parses_openrouter_model_metadata() {
        let provider = ProviderSummary {
            id: "openrouter".into(),
            name: "OpenRouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            protocol: WireProtocol::OpenAi,
            credential_status: crate::models::CredentialStatus::Configured,
            supports_discovery: true,
            configurable_endpoint: false,
            timeout: Default::default(),
        };
        let catalog = json!({"data": [{
            "id": "vendor/reasoning-vision",
            "name": "Reasoning Vision",
            "context_length": 200000,
            "pricing": {"prompt": "0.000002", "completion": "0.000008"},
            "architecture": {"input_modalities": ["text", "image"]}
        }]});
        let models = parse_model_catalog(&provider, &catalog);
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].context_window, 200_000);
        assert_eq!(models[0].input_per_million, Some(2.0));
        assert!(models[0].supports_vision);
        assert!(models[0].reasoning);
    }

    #[test]
    fn parses_anthropic_model_metadata() {
        let provider = ProviderSummary {
            id: "anthropic".into(),
            name: "Anthropic".into(),
            base_url: "https://api.anthropic.com/v1".into(),
            protocol: WireProtocol::Anthropic,
            credential_status: crate::models::CredentialStatus::Configured,
            supports_discovery: true,
            configurable_endpoint: false,
            timeout: Default::default(),
        };
        let models = parse_model_catalog(
            &provider,
            &json!({"data": [{"id": "claude-sonnet-5", "display_name": "Claude Sonnet 5"}]}),
        );
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "claude-sonnet-5");
        assert_eq!(models[0].name, "Claude Sonnet 5");
    }

    #[tokio::test]
    async fn retrieves_models_from_an_anthropic_catalog() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .and(header("x-api-key", "test-secret"))
            .and(header("anthropic-version", "2023-06-01"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": [
                {"id": "claude-sonnet-5", "display_name": "Claude Sonnet 5"}
            ]})))
            .mount(&server)
            .await;
        let provider = ProviderSummary {
            id: "anthropic".into(),
            name: "Anthropic".into(),
            base_url: format!("{}/v1", server.uri()),
            protocol: WireProtocol::Anthropic,
            credential_status: crate::models::CredentialStatus::Configured,
            supports_discovery: true,
            configurable_endpoint: false,
            timeout: Default::default(),
        };
        let models = discover_models(&provider, "test-secret").await.unwrap();
        assert_eq!(models[0].id, "claude-sonnet-5");
    }

    #[test]
    fn filters_non_generation_openai_models() {
        assert!(!is_generation_model("text-embedding-3-large"));
        assert!(!is_generation_model("whisper-1"));
        assert!(is_generation_model("gpt-5-mini"));
    }

    #[tokio::test]
    async fn retrieves_models_from_an_openai_compatible_catalog() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .and(header("authorization", "Bearer test-secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": [
                {"id": "gpt-current", "name": "GPT Current", "context_length": 256000},
                {"id": "text-embedding-3-large"}
            ]})))
            .mount(&server)
            .await;
        let provider = ProviderSummary {
            id: "custom".into(),
            name: "Custom".into(),
            base_url: format!("{}/v1", server.uri()),
            protocol: WireProtocol::OpenAi,
            credential_status: crate::models::CredentialStatus::Configured,
            supports_discovery: true,
            configurable_endpoint: true,
            timeout: Default::default(),
        };
        let models = discover_models(&provider, "test-secret").await.unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "gpt-current");
        assert_eq!(models[0].context_window, 256_000);
    }

    #[tokio::test]
    async fn shared_live_client_completes_two_guarded_streams() {
        let server = MockServer::start().await;
        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"Live \"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"response\"}}]}\n\n",
            "data: [DONE]\n\n"
        );
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("authorization", "Bearer test-secret"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
            .expect(2)
            .mount(&server)
            .await;
        let config = ProviderSummary {
            id: "live-test".into(),
            name: "Live Test".into(),
            base_url: format!("{}/v1", server.uri()),
            protocol: WireProtocol::OpenAi,
            credential_status: crate::models::CredentialStatus::Valid,
            supports_discovery: true,
            configurable_endpoint: true,
            timeout: Default::default(),
        };
        let client = Arc::new(HttpProvider::new(config, "test-secret".into()).unwrap());
        let gate = Arc::new(tokio::sync::Semaphore::new(1));
        let run = |model: &'static str| {
            let client = client.clone();
            let gate = gate.clone();
            async move {
                let _permit = gate.acquire_owned().await.unwrap();
                client
                    .complete(
                        CompletionRequest {
                            system: "Test".into(),
                            prompt: "Test".into(),
                            model: model.into(),
                            max_tokens: 16,
                            temperature: 0.0,
                            thinking_enabled: None,
                            images: vec![],
                        },
                        CancellationToken::new(),
                    )
                    .await
            }
        };
        let (first, second) = tokio::join!(run("model-a"), run("model-b"));
        assert_eq!(first.unwrap().content, "Live response");
        assert_eq!(second.unwrap().content, "Live response");
    }

    #[tokio::test]
    async fn deepseek_stream_ignores_reasoning_and_keeps_final_text() {
        let server = MockServer::start().await;
        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"private\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"OK\"}}]}\n\n",
            "data: [DONE]\n\n"
        );
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
            .mount(&server)
            .await;
        let provider = ProviderSummary {
            id: "deepseek".into(),
            name: "DeepSeek".into(),
            base_url: server.uri(),
            protocol: WireProtocol::OpenAi,
            credential_status: crate::models::CredentialStatus::Valid,
            supports_discovery: false,
            configurable_endpoint: false,
            timeout: Default::default(),
        };
        let client = HttpProvider::new(provider, "test-secret".into()).unwrap();
        let result = client
            .complete(
                CompletionRequest {
                    system: "Connection check".into(),
                    prompt: "OK".into(),
                    model: "deepseek-v4-flash".into(),
                    max_tokens: 256,
                    temperature: 0.0,
                    thinking_enabled: Some(false),
                    images: vec![],
                },
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(result.content, "OK");
        assert!(!result.content.contains("private"));
    }

    #[tokio::test]
    async fn accepts_full_json_when_a_compatible_provider_ignores_streaming() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "choices": [{"message": {"content": "Fallback JSON"}}],
                "usage": {"prompt_tokens": 3, "completion_tokens": 2}
            })))
            .mount(&server)
            .await;
        let provider = ProviderSummary {
            id: "custom".into(),
            name: "Custom".into(),
            base_url: format!("{}/v1", server.uri()),
            protocol: WireProtocol::OpenAi,
            credential_status: crate::models::CredentialStatus::Valid,
            supports_discovery: false,
            configurable_endpoint: true,
            timeout: Default::default(),
        };
        let client = HttpProvider::new(provider, "test-secret".into()).unwrap();
        let result = client
            .complete(
                CompletionRequest {
                    system: "Test".into(),
                    prompt: "Test".into(),
                    model: "custom-model".into(),
                    max_tokens: 32,
                    temperature: 0.0,
                    thinking_enabled: None,
                    images: vec![],
                },
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(result.content, "Fallback JSON");
        assert_eq!(result.input_tokens, Some(3));
        assert_eq!(result.output_tokens, Some(2));
    }
}
