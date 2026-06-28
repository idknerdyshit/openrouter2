use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::Url;

use crate::OpenRouterError;

pub const DEFAULT_BASE_URL: &str = "https://openrouter.ai/api/v1";

pub type QueryParams = Vec<(String, String)>;

pub fn normalize_base_url(raw: impl Into<String>) -> Result<Url, String> {
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
    base_url
        .join(path.trim_start_matches('/'))
        .map_err(|e| OpenRouterError::InvalidBaseUrl(e.to_string()))
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
    use super::{endpoint_url_from_base, normalize_base_url};

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
}
