use std::collections::BTreeMap;
use std::error::Error as _;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::observability::{redact_header_value, redact_reqwest_error_message, redact_text};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct OpenRouterApiError {
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default, rename = "type")]
    pub type_: Option<String>,
    #[serde(default)]
    pub code: Option<Value>,
    #[serde(default)]
    pub param: Option<String>,
    #[serde(default, flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiError {
    pub status: u16,
    pub body: String,
    pub error: Option<OpenRouterApiError>,
    pub request_id: Option<String>,
    pub headers: BTreeMap<String, String>,
}

#[derive(Debug, thiserror::Error)]
pub enum OpenRouterError {
    #[error("invalid openrouter base url: {0}")]
    InvalidBaseUrl(String),
    #[error("invalid request header: {0}")]
    InvalidHeader(String),
    #[error("http transport error: {0}")]
    Transport(String),
    #[error("openrouter api error: status {}", .0.status)]
    Api(Box<ApiError>),
    #[error("malformed openrouter response: {0}")]
    Decode(String),
}

pub(crate) fn reqwest_error_message(e: &reqwest::Error) -> String {
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
    redact_reqwest_error_message(e, msg)
}

pub(crate) fn truncate(s: String) -> String {
    const MAX: usize = 2048;
    if s.len() <= MAX {
        return s;
    }

    let end = floor_char_boundary(&s, MAX);
    let mut t = s;
    t.truncate(end);
    t.push('…');
    t
}

fn floor_char_boundary(s: &str, max: usize) -> usize {
    let mut end = max.min(s.len());
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    end
}

pub(crate) fn parse_api_error(
    status: reqwest::StatusCode,
    headers: &reqwest::header::HeaderMap,
    body: String,
) -> OpenRouterError {
    let body = truncate(redact_text(&body));
    let error = serde_json::from_str::<Value>(&body)
        .ok()
        .and_then(|value| value.get("error").cloned())
        .and_then(|value| serde_json::from_value::<OpenRouterApiError>(value).ok());

    OpenRouterError::Api(Box::new(ApiError {
        status: status.as_u16(),
        body,
        error,
        request_id: header_value(headers, "x-request-id")
            .or_else(|| header_value(headers, "openrouter-request-id")),
        headers: stringify_headers(headers),
    }))
}

fn header_value(headers: &reqwest::header::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(|value| redact_header_value(name, value))
}

fn stringify_headers(headers: &reqwest::header::HeaderMap) -> BTreeMap<String, String> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            let name = name.to_string();
            let value = redact_header_value(&name, value.to_str().ok()?);
            Some((name, value))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use reqwest::StatusCode;
    use reqwest::header::{HeaderMap, HeaderValue};

    use super::{OpenRouterError, parse_api_error, truncate};

    #[test]
    fn truncate_below_max_is_unchanged() {
        let s = "short body".to_owned();
        assert_eq!(truncate(s.clone()), s);
    }

    #[test]
    fn truncate_does_not_split_a_multibyte_char() {
        let mut s = "a".repeat(2047);
        s.push('€');
        let out = truncate(s);
        assert!(out.ends_with('…'));
        assert!(out.len() <= 2047 + '…'.len_utf8());
    }

    #[test]
    fn api_error_redacts_sensitive_headers_body_and_parsed_error() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer sk-test-secret"),
        );
        headers.insert("set-cookie", HeaderValue::from_static("session=secret"));
        headers.insert(
            "x-request-id",
            HeaderValue::from_static("req-sk-test-secret"),
        );

        let err = parse_api_error(
            StatusCode::UNAUTHORIZED,
            &headers,
            r#"{"error":{"message":"bad Bearer sk-other-secret"},"key":"sk-test-secret"}"#
                .to_owned(),
        );

        let OpenRouterError::Api(api) = err else {
            panic!("expected api error");
        };

        assert_eq!(api.request_id.as_deref(), Some("req-[REDACTED]"));
        assert_eq!(
            api.headers.get("authorization").map(String::as_str),
            Some("[REDACTED]")
        );
        assert_eq!(
            api.headers.get("set-cookie").map(String::as_str),
            Some("[REDACTED]")
        );
        assert!(!api.body.contains("sk-test-secret"));
        assert!(!api.body.contains("sk-other-secret"));
        assert_eq!(
            api.error.and_then(|error| error.message).as_deref(),
            Some("bad Bearer [REDACTED]")
        );
    }
}
