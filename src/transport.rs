use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::Url;

use crate::OpenRouterError;

pub const DEFAULT_BASE_URL: &str = "https://openrouter.ai/api/v1";

pub type QueryParams = Vec<(String, String)>;

pub trait IntoQueryParams {
    fn into_query_params(self) -> QueryParams;
}

impl IntoQueryParams for QueryParams {
    fn into_query_params(self) -> QueryParams {
        self
    }
}

impl IntoQueryParams for () {
    fn into_query_params(self) -> QueryParams {
        Vec::new()
    }
}

impl<const N: usize> IntoQueryParams for [(&str, &str); N] {
    fn into_query_params(self) -> QueryParams {
        self.into_iter()
            .map(|(key, value)| (key.to_owned(), value.to_owned()))
            .collect()
    }
}

pub fn normalize_base_url(raw: impl Into<String>) -> Result<Url, String> {
    let url = normalize_unchecked_base_url(raw)?;
    validate_trusted_base_url(&url)?;
    Ok(url)
}

pub fn normalize_unchecked_base_url(raw: impl Into<String>) -> Result<Url, String> {
    let raw = raw.into();
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

pub fn endpoint_url_from_base(base_url: &Url, path: &str) -> Result<Url, OpenRouterError> {
    let path = relative_endpoint_path(path).map_err(OpenRouterError::InvalidBaseUrl)?;
    base_url
        .join(path)
        .map_err(|e| OpenRouterError::InvalidBaseUrl(e.to_string()))
}

fn validate_trusted_base_url(url: &Url) -> Result<(), String> {
    if url.scheme() != "https" {
        return Err(
            "base URL must use https; use the unchecked custom-base constructor for tests or proxies"
                .to_owned(),
        );
    }

    let host = url
        .host_str()
        .ok_or_else(|| "base URL must include a host".to_owned())?;
    if host.eq_ignore_ascii_case("openrouter.ai") || host.ends_with(".openrouter.ai") {
        Ok(())
    } else {
        Err(
            "base URL host must be openrouter.ai; use the unchecked custom-base constructor for tests or proxies"
                .to_owned(),
        )
    }
}

fn relative_endpoint_path(path: &str) -> Result<&str, String> {
    let trimmed = path.trim();
    if trimmed != path {
        return Err("request path must not include leading or trailing whitespace".to_owned());
    }
    if trimmed.is_empty() {
        return Err("request path is empty".to_owned());
    }
    if trimmed.starts_with("//") {
        return Err("request path must be relative and must not include an authority".to_owned());
    }
    if trimmed.contains('?') || trimmed.contains('#') {
        return Err("request path must not include a query string or fragment".to_owned());
    }

    let relative = trimmed.trim_start_matches('/');
    if relative.is_empty() {
        return Err("request path is empty".to_owned());
    }
    if relative
        .split('/')
        .next()
        .is_some_and(|segment| segment.contains(':'))
    {
        return Err("request path must be relative and must not include a scheme".to_owned());
    }

    Ok(relative)
}

pub(crate) fn path_segment(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}

pub(crate) fn with_query(mut url: Url, query: &[(String, String)]) -> Url {
    if !query.is_empty() {
        url.query_pairs_mut().extend_pairs(
            query
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str())),
        );
    }
    url
}

#[cfg(test)]
mod tests {
    use super::{endpoint_url_from_base, normalize_base_url, normalize_unchecked_base_url};

    #[test]
    fn normalizes_base_url_with_trailing_slash() {
        let url = normalize_base_url(" https://openrouter.ai/api/v1/ ").unwrap();
        assert_eq!(url.as_str(), "https://openrouter.ai/api/v1/");

        let url = normalize_base_url("https://openrouter.ai/api/v1").unwrap();
        assert_eq!(url.as_str(), "https://openrouter.ai/api/v1/");
    }

    #[test]
    fn rejects_unusable_base_urls() {
        assert!(normalize_base_url("").is_err());
        assert!(normalize_base_url("mailto:ops@example.test").is_err());
        assert!(normalize_base_url("https://openrouter.ai/api/v1?x=1").is_err());
        assert!(normalize_base_url("https://openrouter.ai/api/v1#frag").is_err());
        assert!(normalize_base_url("http://openrouter.ai/api/v1").is_err());
        assert!(normalize_base_url("https://proxy.example.test/api/v1").is_err());
    }

    #[test]
    fn unchecked_base_url_supports_explicit_custom_hosts() {
        let url = normalize_unchecked_base_url("http://127.0.0.1:1234/api").unwrap();
        assert_eq!(url.as_str(), "http://127.0.0.1:1234/api/");
    }

    #[test]
    fn endpoint_join_preserves_configured_base_path() {
        let base_url = normalize_base_url("https://openrouter.ai/custom/openrouter").unwrap();
        assert_eq!(
            endpoint_url_from_base(&base_url, "chat/completions")
                .unwrap()
                .as_str(),
            "https://openrouter.ai/custom/openrouter/chat/completions"
        );
    }

    #[test]
    fn endpoint_join_rejects_absolute_or_query_paths() {
        let base_url = normalize_base_url("https://openrouter.ai/api/v1").unwrap();
        assert!(endpoint_url_from_base(&base_url, "https://attacker.test/x").is_err());
        assert!(endpoint_url_from_base(&base_url, "//attacker.test/x").is_err());
        assert!(endpoint_url_from_base(&base_url, "chat/completions?api_key=secret").is_err());
        assert!(endpoint_url_from_base(&base_url, "chat/completions#secret").is_err());
    }
}
