//! `openrouter2` is a thin async `reqwest` client over the OpenRouter API.
//!
//! It exposes two focused surfaces: [`OpenRouterClient::chat_completion`] for
//! `POST /chat/completions` and [`OpenRouterClient::generation_cost`] for
//! `GET /generation?id=...`.
//!
//! The HTTP client is injected so callers can share one `reqwest::Client` for
//! connection-pool and TLS reuse. API keys are per-call arguments and are never
//! stored on the client value.

use std::error::Error as _;

use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Default OpenRouter API base (no trailing slash).
pub const DEFAULT_BASE_URL: &str = "https://openrouter.ai/api/v1";

#[derive(Debug, thiserror::Error)]
pub enum OpenRouterError {
    #[error("invalid openrouter base url: {0}")]
    InvalidBaseUrl(String),
    #[error("http transport error: {0}")]
    Transport(String),
    /// Non-2xx response. `status` is the HTTP code; `body` is the (truncated)
    /// response text for diagnostics — it must never carry the API key.
    #[error("openrouter api error: status {status}")]
    Api { status: u16, body: String },
    #[error("malformed openrouter response: {0}")]
    Decode(String),
}

/// One chat message (role + content). Roles are OpenRouter/OpenAI conventions:
/// `system`, `user`, `assistant`.
#[derive(Debug, Clone, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_owned(),
            content: content.into(),
        }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_owned(),
            content: content.into(),
        }
    }
}

/// A chat-completion request. Only the deterministic knobs we use are modelled.
#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub response_format: Option<Value>,
    pub provider: Option<ProviderPreferences>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderPreferences {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_parameters: Option<bool>,
}

/// The parsed chat-completion result: the assistant text, the model that served
/// it, the generation id (to query cost later), and the usage token counts.
#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub content: String,
    pub model: String,
    pub generation_id: Option<String>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
}

// --- wire types (OpenRouter chat-completions response) ---------------------

#[derive(Serialize)]
struct WireRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<&'a Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider: Option<&'a ProviderPreferences>,
}

#[derive(Deserialize)]
struct WireResponse {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    choices: Vec<WireChoice>,
    #[serde(default)]
    usage: Option<WireUsage>,
}

#[derive(Deserialize)]
struct WireChoice {
    #[serde(default)]
    message: Option<WireMessage>,
}

#[derive(Deserialize)]
struct WireMessage {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Deserialize)]
struct WireUsage {
    #[serde(default)]
    prompt_tokens: Option<i32>,
    #[serde(default)]
    completion_tokens: Option<i32>,
}

// --- wire types (OpenRouter GET /generation cost lookup) -------------------

#[derive(Deserialize)]
struct GenerationEnvelope {
    #[serde(default)]
    data: Option<GenerationData>,
}

#[derive(Deserialize)]
struct GenerationData {
    /// Authoritative total cost in USD. It can be absent immediately after a
    /// generation because the OpenRouter endpoint is eventually consistent.
    #[serde(default)]
    total_cost: Option<f64>,
}

/// Truncate a response body for error reporting (defense-in-depth: keep error
/// logs bounded; the key is never in the body).
fn truncate(s: String) -> String {
    const MAX: usize = 2048;
    if s.len() > MAX {
        // `String::truncate` panics unless the byte offset is a char boundary, so
        // back off to the largest boundary at or below MAX — a multibyte char
        // straddling the cut (common in non-ASCII error bodies) must not crash
        // error handling.
        let end = floor_char_boundary(&s, MAX);
        let mut t = s;
        t.truncate(end);
        t.push('…');
        t
    } else {
        s
    }
}

fn floor_char_boundary(s: &str, max: usize) -> usize {
    let mut end = max.min(s.len());
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    end
}

pub struct OpenRouterClient {
    http: reqwest::Client,
    base_url: Url,
}

impl OpenRouterClient {
    /// Build over an injected (shared) `reqwest::Client`. `base_url` has no
    /// trailing slash (see [`DEFAULT_BASE_URL`]).
    pub fn new(http: reqwest::Client, base_url: impl Into<String>) -> Self {
        Self::try_new(http, base_url).expect("invalid OpenRouter base URL")
    }

    /// Fallible constructor for configuration paths that want to surface a typed
    /// startup error instead of panicking.
    pub fn try_new(
        http: reqwest::Client,
        base_url: impl Into<String>,
    ) -> Result<Self, OpenRouterError> {
        Ok(Self {
            http,
            base_url: normalize_base_url(base_url.into())
                .map_err(OpenRouterError::InvalidBaseUrl)?,
        })
    }

    /// Call chat-completions with a per-org API key. The key goes in the
    /// `Authorization` header only — never logged, never in an error body.
    pub async fn chat_completion(
        &self,
        api_key: &str,
        req: ChatRequest,
    ) -> Result<ChatResponse, OpenRouterError> {
        let wire = WireRequest {
            model: &req.model,
            messages: &req.messages,
            temperature: req.temperature,
            max_tokens: req.max_tokens,
            response_format: req.response_format.as_ref(),
            provider: req.provider.as_ref(),
        };

        let resp = self
            .http
            .post(self.endpoint_url("chat/completions")?)
            .bearer_auth(api_key)
            .json(&wire)
            .send()
            .await
            .map_err(|e| OpenRouterError::Transport(reqwest_error_message(&e)))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(OpenRouterError::Api {
                status: status.as_u16(),
                body: truncate(body),
            });
        }

        let parsed: WireResponse = resp
            .json()
            .await
            .map_err(|e| OpenRouterError::Decode(e.to_string()))?;

        let content = parsed
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message)
            .and_then(|m| m.content)
            .ok_or_else(|| OpenRouterError::Decode("no choices/message content".to_owned()))?;

        let (prompt_tokens, completion_tokens) = parsed
            .usage
            .map(|u| (u.prompt_tokens, u.completion_tokens))
            .unwrap_or((None, None));

        Ok(ChatResponse {
            content,
            model: parsed.model.unwrap_or(req.model),
            generation_id: parsed.id,
            prompt_tokens,
            completion_tokens,
        })
    }

    /// Look up the authoritative cost for a generation id (`GET /generation?id=...`).
    /// Returns `Ok(None)` when the cost is not yet available (eventually
    /// consistent), so callers can retry later.
    pub async fn generation_cost(
        &self,
        api_key: &str,
        generation_id: &str,
    ) -> Result<Option<f64>, OpenRouterError> {
        let resp = self
            .http
            .get(self.endpoint_url("generation")?)
            .query(&[("id", generation_id)])
            .bearer_auth(api_key)
            .send()
            .await
            .map_err(|e| OpenRouterError::Transport(reqwest_error_message(&e)))?;

        let status = resp.status();
        // 404 = the id isn't queryable yet (eventual consistency), so treat it
        // as "not available" rather than as a hard error.
        if status.as_u16() == 404 {
            return Ok(None);
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(OpenRouterError::Api {
                status: status.as_u16(),
                body: truncate(body),
            });
        }

        let envelope: GenerationEnvelope = resp
            .json()
            .await
            .map_err(|e| OpenRouterError::Decode(e.to_string()))?;
        Ok(envelope.data.and_then(|d| d.total_cost))
    }

    fn endpoint_url(&self, path: &str) -> Result<Url, OpenRouterError> {
        endpoint_url_from_base(&self.base_url, path)
    }
}

fn normalize_base_url(raw: String) -> Result<Url, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("base URL is empty".to_owned());
    }

    let mut url = Url::parse(trimmed).map_err(|e| e.to_string())?;
    match url.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(format!(
                "unsupported scheme {scheme:?}; expected http or https"
            ));
        }
    }
    if url.host_str().is_none() {
        return Err("base URL must include a host".to_owned());
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err("base URL must not include a query string or fragment".to_owned());
    }
    if !url.path().ends_with('/') {
        let path = format!("{}/", url.path());
        url.set_path(&path);
    }
    Ok(url)
}

fn endpoint_url_from_base(base_url: &Url, path: &str) -> Result<Url, OpenRouterError> {
    base_url
        .join(path)
        .map_err(|e| OpenRouterError::InvalidBaseUrl(e.to_string()))
}

fn reqwest_error_message(e: &reqwest::Error) -> String {
    let mut msg = e.to_string();
    if let Some(status) = e.status()
        && !msg.contains(status.as_str())
    {
        msg = format!("status {status}: {msg}");
    }
    if let Some(source) = e.source() {
        let source = source.to_string();
        if !source.is_empty() && !msg.contains(&source) {
            msg = format!("{msg}: {source}");
        }
    }
    msg
}

#[cfg(test)]
mod tests {
    use super::{
        ChatMessage, ChatRequest, OpenRouterClient, OpenRouterError, ProviderPreferences,
        WireRequest, endpoint_url_from_base, normalize_base_url, truncate,
    };
    use serde_json::Value;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::oneshot;

    #[derive(Debug)]
    struct RecordedRequest {
        method: String,
        path: String,
        headers: Vec<(String, String)>,
        body: String,
    }

    impl RecordedRequest {
        fn header(&self, name: &str) -> Option<&str> {
            self.headers
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(name))
                .map(|(_, value)| value.as_str())
        }
    }

    async fn serve_once(
        status: &'static str,
        body: impl Into<String>,
    ) -> (String, oneshot::Receiver<RecordedRequest>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let body = body.into();
        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let request = read_request(&mut stream).await.unwrap();
            let _ = tx.send(request);

            let response = format!(
                "HTTP/1.1 {status}\r\n\
                 content-type: application/json\r\n\
                 content-length: {}\r\n\
                 connection: close\r\n\
                 \r\n\
                 {body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).await.unwrap();
        });

        (format!("http://{addr}"), rx)
    }

    async fn read_request(stream: &mut TcpStream) -> std::io::Result<RecordedRequest> {
        let mut buf = Vec::new();
        let mut header_end = None;
        let mut content_length = 0;

        loop {
            let mut chunk = [0_u8; 1024];
            let n = stream.read(&mut chunk).await?;
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);

            if header_end.is_none() {
                if let Some(end) = find_header_end(&buf) {
                    header_end = Some(end);
                    content_length = parse_content_length(&buf[..end]);
                }
            }

            if let Some(end) = header_end {
                if buf.len() >= end + 4 + content_length {
                    break;
                }
            }
        }

        let header_end = header_end.unwrap_or(buf.len());
        let header_text = String::from_utf8_lossy(&buf[..header_end]);
        let mut lines = header_text.lines();
        let request_line = lines.next().unwrap_or_default();
        let mut request_parts = request_line.split_whitespace();
        let method = request_parts.next().unwrap_or_default().to_owned();
        let path = request_parts.next().unwrap_or_default().to_owned();
        let headers = lines
            .filter_map(|line| {
                let (name, value) = line.split_once(':')?;
                Some((name.trim().to_owned(), value.trim().to_owned()))
            })
            .collect::<Vec<_>>();

        let body_start = header_end + 4;
        let body_end = body_start + content_length;
        let body = if body_end <= buf.len() {
            String::from_utf8_lossy(&buf[body_start..body_end]).into_owned()
        } else {
            String::new()
        };

        Ok(RecordedRequest {
            method,
            path,
            headers,
            body,
        })
    }

    fn find_header_end(buf: &[u8]) -> Option<usize> {
        buf.windows(4).position(|window| window == b"\r\n\r\n")
    }

    fn parse_content_length(headers: &[u8]) -> usize {
        String::from_utf8_lossy(headers)
            .lines()
            .skip(1)
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                if name.eq_ignore_ascii_case("content-length") {
                    value.trim().parse().ok()
                } else {
                    None
                }
            })
            .unwrap_or(0)
    }

    fn sample_chat_request() -> ChatRequest {
        ChatRequest {
            model: "openai/gpt-4o-mini".to_owned(),
            messages: vec![
                ChatMessage::system("You are concise."),
                ChatMessage::user("Say hello."),
            ],
            temperature: Some(0.2),
            max_tokens: Some(64),
            response_format: None,
            provider: None,
        }
    }

    #[test]
    fn truncate_below_max_is_unchanged() {
        let s = "short body".to_owned();
        assert_eq!(truncate(s.clone()), s);
    }

    // A multibyte char straddling the 2048-byte cut must not panic; we back off to
    // the nearest char boundary and append the ellipsis.
    #[test]
    fn truncate_does_not_split_a_multibyte_char() {
        // 2047 ASCII bytes + a 3-byte '€' = 2050 bytes; the naive cut at 2048 lands
        // mid-'€' (not a char boundary) and would panic.
        let mut s = "a".repeat(2047);
        s.push('€');
        let out = truncate(s); // must not panic
        assert!(out.ends_with('…'));
        assert!(out.len() <= 2047 + '…'.len_utf8());
    }

    #[test]
    fn normalizes_base_url_with_trailing_slash() {
        let url = normalize_base_url(" https://openrouter.ai/api/v1/ ".to_owned()).unwrap();
        assert_eq!(url.as_str(), "https://openrouter.ai/api/v1/");

        let url = normalize_base_url("https://openrouter.ai/api/v1".to_owned()).unwrap();
        assert_eq!(url.as_str(), "https://openrouter.ai/api/v1/");
    }

    #[test]
    fn rejects_unusable_base_urls() {
        assert!(normalize_base_url("".to_owned()).is_err());
        assert!(normalize_base_url("mailto:ops@example.test".to_owned()).is_err());
        assert!(normalize_base_url("https://openrouter.ai/api/v1?x=1".to_owned()).is_err());
        assert!(normalize_base_url("https://openrouter.ai/api/v1#frag".to_owned()).is_err());
    }

    #[test]
    fn endpoint_join_preserves_configured_base_path() {
        let base_url =
            normalize_base_url("https://openrouter.ai/custom/openrouter".to_owned()).unwrap();
        assert_eq!(
            endpoint_url_from_base(&base_url, "chat/completions")
                .unwrap()
                .as_str(),
            "https://openrouter.ai/custom/openrouter/chat/completions"
        );
        assert_eq!(
            endpoint_url_from_base(&base_url, "generation")
                .unwrap()
                .as_str(),
            "https://openrouter.ai/custom/openrouter/generation"
        );
    }

    #[test]
    fn serializes_classifier_structured_output_options() {
        let response_format = serde_json::json!({
            "type": "json_schema",
            "json_schema": {
                "name": "inbound_intent",
                "strict": true,
                "schema": { "type": "object" }
            }
        });
        let provider = ProviderPreferences {
            require_parameters: Some(true),
        };
        let messages = vec![ChatMessage::user("Return JSON only.")];
        let wire = WireRequest {
            model: "openai/gpt-5-nano",
            messages: &messages,
            temperature: Some(0.0),
            max_tokens: Some(256),
            response_format: Some(&response_format),
            provider: Some(&provider),
        };

        let value = serde_json::to_value(wire).unwrap();
        assert_eq!(value["response_format"]["type"], "json_schema");
        assert_eq!(value["response_format"]["json_schema"]["strict"], true);
        assert_eq!(value["provider"]["require_parameters"], true);
    }

    #[test]
    fn omits_structured_output_options_for_normal_drafts() {
        let messages = vec![ChatMessage::user("Draft a reply.")];
        let wire = WireRequest {
            model: "openai/gpt-4o-mini",
            messages: &messages,
            temperature: Some(0.2),
            max_tokens: Some(240),
            response_format: None,
            provider: None,
        };

        let value = serde_json::to_value(wire).unwrap();
        assert!(value.get("response_format").is_none());
        assert!(value.get("provider").is_none());
    }

    #[tokio::test]
    async fn chat_completion_posts_expected_json_and_parses_response() {
        let (base_url, request) = serve_once(
            "200 OK",
            r#"{"id":"gen-123","model":"openai/gpt-4o-mini","choices":[{"message":{"content":"hello there"}}],"usage":{"prompt_tokens":12,"completion_tokens":3}}"#,
        )
        .await;
        let client = OpenRouterClient::try_new(reqwest::Client::new(), base_url).unwrap();

        let response = client
            .chat_completion("sk-test", sample_chat_request())
            .await
            .unwrap();

        assert_eq!(response.content, "hello there");
        assert_eq!(response.model, "openai/gpt-4o-mini");
        assert_eq!(response.generation_id.as_deref(), Some("gen-123"));
        assert_eq!(response.prompt_tokens, Some(12));
        assert_eq!(response.completion_tokens, Some(3));

        let recorded = request.await.unwrap();
        assert_eq!(recorded.method, "POST");
        assert_eq!(recorded.path, "/chat/completions");
        assert_eq!(recorded.header("authorization"), Some("Bearer sk-test"));
        assert!(!recorded.body.contains("sk-test"));

        let body: Value = serde_json::from_str(&recorded.body).unwrap();
        assert_eq!(body["model"], "openai/gpt-4o-mini");
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][1]["content"], "Say hello.");
        assert_eq!(body["temperature"], 0.2);
        assert_eq!(body["max_tokens"], 64);
    }

    #[tokio::test]
    async fn chat_completion_api_error_truncates_body_without_key() {
        let api_key = "sk-test-secret";
        let (base_url, request) = serve_once("500 Internal Server Error", "x".repeat(3000)).await;
        let client = OpenRouterClient::try_new(reqwest::Client::new(), base_url).unwrap();

        let err = client
            .chat_completion(api_key, sample_chat_request())
            .await
            .unwrap_err();

        match err {
            OpenRouterError::Api { status, body } => {
                assert_eq!(status, 500);
                assert!(body.len() < 3000);
                assert!(body.ends_with('…'));
                assert!(!body.contains(api_key));
            }
            other => panic!("expected api error, got {other:?}"),
        }

        let recorded = request.await.unwrap();
        assert!(!recorded.body.contains(api_key));
    }

    #[tokio::test]
    async fn generation_cost_returns_some_for_available_cost() {
        let (base_url, request) = serve_once("200 OK", r#"{"data":{"total_cost":0.000123}}"#).await;
        let client = OpenRouterClient::try_new(reqwest::Client::new(), base_url).unwrap();

        let cost = client
            .generation_cost("sk-cost", "gen-456")
            .await
            .unwrap()
            .unwrap();

        assert!((cost - 0.000123).abs() < f64::EPSILON);

        let recorded = request.await.unwrap();
        assert_eq!(recorded.method, "GET");
        assert_eq!(recorded.path, "/generation?id=gen-456");
        assert_eq!(recorded.header("authorization"), Some("Bearer sk-cost"));
        assert!(recorded.body.is_empty());
    }

    #[tokio::test]
    async fn generation_cost_returns_none_for_not_yet_queryable_generation() {
        let (base_url, request) = serve_once("404 Not Found", "{}").await;
        let client = OpenRouterClient::try_new(reqwest::Client::new(), base_url).unwrap();

        let cost = client.generation_cost("sk-cost", "gen-789").await.unwrap();

        assert_eq!(cost, None);

        let recorded = request.await.unwrap();
        assert_eq!(recorded.method, "GET");
        assert_eq!(recorded.path, "/generation?id=gen-789");
        assert_eq!(recorded.header("authorization"), Some("Bearer sk-cost"));
    }
}
