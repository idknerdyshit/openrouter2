use std::time::Instant;

use reqwest::header::HeaderMap;
use reqwest::{StatusCode, Url};
use serde_json::Value;

use crate::routes::HttpMethod;
use crate::spec::NON_DEPRECATED_ROUTES;

const REDACTED: &str = "[REDACTED]";

#[derive(Debug)]
pub(crate) struct RequestTrace {
    method: HttpMethod,
    route: String,
    query_count: usize,
    authenticated: bool,
    started_at: Instant,
}

impl RequestTrace {
    pub(crate) fn start(
        method: HttpMethod,
        path: &str,
        query: &[(String, String)],
        authenticated: bool,
    ) -> Self {
        let route = route_template(method, path)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| redact_path(path));
        let trace = Self {
            method,
            route,
            query_count: query.len(),
            authenticated,
            started_at: Instant::now(),
        };

        tracing::debug!(
            target: "openrouter2::http",
            method = method_as_str(trace.method),
            route = %trace.route,
            query_count = trace.query_count,
            authenticated = trace.authenticated,
            "openrouter request started"
        );

        trace
    }

    pub(crate) fn response(&self, status: StatusCode, headers: &HeaderMap) {
        let status_code = status.as_u16();
        let elapsed_ms = elapsed_ms(self.started_at);
        let request_id = request_id(headers).unwrap_or_default();

        if status.is_success() {
            tracing::debug!(
                target: "openrouter2::http",
                method = method_as_str(self.method),
                route = %self.route,
                status = status_code,
                elapsed_ms = elapsed_ms,
                request_id = request_id,
                "openrouter request completed"
            );
        } else {
            tracing::warn!(
                target: "openrouter2::http",
                method = method_as_str(self.method),
                route = %self.route,
                status = status_code,
                elapsed_ms = elapsed_ms,
                request_id = request_id,
                "openrouter request failed"
            );
        }
    }

    pub(crate) fn transport_error(&self, error: &reqwest::Error) {
        let elapsed_ms = elapsed_ms(self.started_at);
        let error = redact_reqwest_error(error);

        tracing::warn!(
            target: "openrouter2::http",
            method = method_as_str(self.method),
            route = %self.route,
            elapsed_ms = elapsed_ms,
            error = %error,
            "openrouter request transport error"
        );
    }
}

pub(crate) fn redact_reqwest_error(error: &reqwest::Error) -> String {
    redact_reqwest_error_message(error, error.to_string())
}

pub(crate) fn redact_reqwest_error_message(error: &reqwest::Error, msg: String) -> String {
    let mut msg = msg;
    if let Some(url) = error.url() {
        msg = msg.replace(url.as_str(), &redact_url(url));
    }
    redact_text(&msg)
}

pub(crate) fn redact_header_value(name: &str, value: &str) -> String {
    if is_sensitive_name(name) {
        REDACTED.to_owned()
    } else {
        redact_secret_patterns(value)
    }
}

pub(crate) fn redact_text(raw: &str) -> String {
    let redacted = match serde_json::from_str::<Value>(raw) {
        Ok(mut value) => {
            redact_json_value(&mut value);
            serde_json::to_string(&value).unwrap_or_else(|_| raw.to_owned())
        }
        Err(_) => raw.to_owned(),
    };

    redact_secret_patterns(&redacted)
}

pub(crate) fn redact_url(url: &Url) -> String {
    let mut redacted = url.clone();

    if !redacted.username().is_empty() {
        let _ = redacted.set_username(REDACTED);
    }
    if redacted.password().is_some() {
        let _ = redacted.set_password(Some(REDACTED));
    }
    if redacted.query().is_some() {
        let keys = redacted
            .query_pairs()
            .map(|(key, _)| key.into_owned())
            .collect::<Vec<_>>();
        redacted
            .query_pairs_mut()
            .clear()
            .extend_pairs(keys.iter().map(|key| (key.as_str(), REDACTED)));
    }

    redacted.to_string()
}

fn redact_json_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, value) in map.iter_mut() {
                if is_sensitive_name(key) {
                    *value = Value::String(REDACTED.to_owned());
                } else {
                    redact_json_value(value);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                redact_json_value(value);
            }
        }
        Value::String(value) => {
            *value = redact_secret_patterns(value);
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn redact_secret_patterns(input: &str) -> String {
    redact_prefixed_tokens(&redact_bearer_tokens(input))
}

fn redact_bearer_tokens(input: &str) -> String {
    let lower = input.to_ascii_lowercase();
    let mut out = String::with_capacity(input.len());
    let mut copied_until = 0;
    let mut search_from = 0;

    while let Some(relative_start) = lower[search_from..].find("bearer ") {
        let marker_start = search_from + relative_start;
        let token_start = marker_start + "bearer ".len();
        let token_end = token_end(input, token_start);

        if token_end == token_start {
            search_from = token_start;
            continue;
        }

        out.push_str(&input[copied_until..token_start]);
        out.push_str(REDACTED);
        copied_until = token_end;
        search_from = token_end;
    }

    out.push_str(&input[copied_until..]);
    out
}

fn redact_prefixed_tokens(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut index = 0;

    while index < input.len() {
        let rest = &input[index..];
        let prefix = if rest.starts_with("sk-or-v1-") {
            Some("sk-or-v1-")
        } else if rest.starts_with("sk-") {
            Some("sk-")
        } else {
            None
        };

        if let Some(prefix) = prefix {
            let end = token_end(input, index);
            if end > index + prefix.len() {
                out.push_str(REDACTED);
                index = end;
                continue;
            }
        }

        let ch = rest
            .chars()
            .next()
            .expect("index is inside a non-empty string slice");
        out.push(ch);
        index += ch.len_utf8();
    }

    out
}

fn token_end(input: &str, token_start: usize) -> usize {
    input[token_start..]
        .char_indices()
        .find_map(|(offset, ch)| (!is_secret_token_char(ch)).then_some(token_start + offset))
        .unwrap_or(input.len())
}

fn is_secret_token_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.')
}

fn is_sensitive_name(name: &str) -> bool {
    let normalized = name
        .chars()
        .filter(|ch| !matches!(ch, '-' | '_' | '.'))
        .flat_map(char::to_lowercase)
        .collect::<String>();

    matches!(
        normalized.as_str(),
        "authorization"
            | "proxyauthorization"
            | "cookie"
            | "setcookie"
            | "apikey"
            | "xapikey"
            | "openrouterapikey"
            | "key"
            | "token"
            | "accesstoken"
            | "refreshtoken"
            | "idtoken"
            | "secret"
            | "clientsecret"
            | "password"
            | "passphrase"
            | "privatekey"
            | "sessionid"
    ) || normalized.ends_with("token")
        || normalized.ends_with("secret")
        || normalized.ends_with("apikey")
        || normalized.ends_with("password")
        || normalized.ends_with("privatekey")
}

fn route_template(method: HttpMethod, path: &str) -> Option<&'static str> {
    let candidate = path_segments(path);
    NON_DEPRECATED_ROUTES
        .iter()
        .find(|spec| spec.method == method_as_str(method) && route_matches(spec.path, &candidate))
        .map(|spec| spec.path)
}

fn route_matches(template: &str, candidate: &[&str]) -> bool {
    let template = path_segments(template);
    template.len() == candidate.len()
        && template
            .iter()
            .zip(candidate)
            .all(|(template, candidate)| is_template_param(template) || template == candidate)
}

fn path_segments(path: &str) -> Vec<&str> {
    path.split(['?', '#'])
        .next()
        .unwrap_or(path)
        .trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn is_template_param(segment: &str) -> bool {
    segment.starts_with('{') && segment.ends_with('}')
}

fn redact_path(path: &str) -> String {
    let (path, query) = path.split_once('?').unwrap_or((path, ""));
    let path = redact_secret_patterns(path);
    if query.is_empty() {
        path
    } else {
        format!("{path}?{}", redact_query(query))
    }
}

fn redact_query(query: &str) -> String {
    query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let key = pair.split_once('=').map_or(pair, |(key, _)| key);
            format!("{}={REDACTED}", redact_secret_patterns(key))
        })
        .collect::<Vec<_>>()
        .join("&")
}

fn request_id(headers: &HeaderMap) -> Option<String> {
    header_value(headers, "x-request-id").or_else(|| header_value(headers, "openrouter-request-id"))
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(|value| redact_header_value(name, value))
}

fn method_as_str(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Get => "GET",
        HttpMethod::Post => "POST",
        HttpMethod::Put => "PUT",
        HttpMethod::Patch => "PATCH",
        HttpMethod::Delete => "DELETE",
    }
}

fn elapsed_ms(started_at: Instant) -> u64 {
    u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use reqwest::Url;

    use super::{RequestTrace, redact_header_value, redact_text, redact_url};
    use crate::routes::HttpMethod;

    #[test]
    fn trace_uses_route_templates_for_known_dynamic_paths() {
        let trace = RequestTrace::start(HttpMethod::Get, "keys/sk-test-secret", &[], true);
        assert_eq!(trace.route, "/keys/{hash}");
        assert!(!trace.route.contains("sk-test-secret"));
    }

    #[test]
    fn trace_redacts_unknown_path_and_query_values() {
        let trace = RequestTrace::start(
            HttpMethod::Get,
            "custom/sk-test-secret?api_key=sk-query-secret&model=openai",
            &[],
            true,
        );
        assert_eq!(
            trace.route,
            "custom/[REDACTED]?api_key=[REDACTED]&model=[REDACTED]"
        );
        assert!(!trace.route.contains("sk-test-secret"));
        assert!(!trace.route.contains("sk-query-secret"));
    }

    #[test]
    fn redacts_sensitive_headers_and_secret_patterns() {
        assert_eq!(
            redact_header_value("authorization", "Bearer sk-test-secret"),
            "[REDACTED]"
        );
        assert_eq!(
            redact_header_value("x-title", "demo sk-test-secret"),
            "demo [REDACTED]"
        );
    }

    #[test]
    fn redacts_json_sensitive_fields_and_embedded_tokens() {
        let redacted = redact_text(
            r#"{"key":"sk-test-secret","error":{"message":"bad Bearer sk-other-secret"}}"#,
        );
        assert_eq!(
            redacted,
            r#"{"error":{"message":"bad Bearer [REDACTED]"},"key":"[REDACTED]"}"#
        );
    }

    #[test]
    fn redacts_url_credentials_and_query_values() {
        let url =
            Url::parse("https://user:pass@example.test/path?api_key=sk-secret&id=gen-123").unwrap();
        let redacted = redact_url(&url);
        assert_eq!(
            redacted,
            "https://%5BREDACTED%5D:%5BREDACTED%5D@example.test/path?api_key=%5BREDACTED%5D&id=%5BREDACTED%5D"
        );
    }
}
