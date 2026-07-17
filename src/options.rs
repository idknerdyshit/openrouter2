use reqwest::header::{HeaderName, HeaderValue};

use crate::{ApiKey, OpenRouterError};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum RequestAuth {
    #[default]
    Default,
    ApiKey(ApiKey),
    NoAuth,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestOptions {
    pub http_referer: Option<String>,
    pub x_title: Option<String>,
    pub x_openrouter_title: Option<String>,
    pub x_openrouter_categories: Option<String>,
    pub x_openrouter_metadata: Option<String>,
    pub session_id: Option<String>,
    pub extra_headers: Vec<(String, String)>,
    pub auth: RequestAuth,
}

impl RequestOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_http_referer(mut self, referer: impl Into<String>) -> Self {
        self.http_referer = Some(referer.into());
        self
    }

    pub fn with_x_title(mut self, title: impl Into<String>) -> Self {
        self.x_title = Some(title.into());
        self
    }

    pub fn with_openrouter_title(mut self, title: impl Into<String>) -> Self {
        self.x_openrouter_title = Some(title.into());
        self
    }

    pub fn with_openrouter_categories(mut self, categories: impl Into<String>) -> Self {
        self.x_openrouter_categories = Some(categories.into());
        self
    }

    pub fn with_openrouter_metadata(mut self, enabled: bool) -> Self {
        self.x_openrouter_metadata = Some(if enabled { "enabled" } else { "disabled" }.to_owned());
        self
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra_headers.push((name.into(), value.into()));
        self
    }

    pub fn with_api_key(mut self, api_key: impl Into<ApiKey>) -> Self {
        self.auth = RequestAuth::ApiKey(api_key.into());
        self
    }

    pub fn without_auth(mut self) -> Self {
        self.auth = RequestAuth::NoAuth;
        self
    }

    #[cfg(feature = "async")]
    pub(crate) fn apply_async(
        &self,
        mut builder: reqwest::RequestBuilder,
    ) -> Result<reqwest::RequestBuilder, OpenRouterError> {
        for (name, value) in &self.extra_headers {
            if has_typed_header(self, name) {
                continue;
            }
            builder = builder.header(header_name(name)?, header_value(value)?);
        }
        if let Some(value) = &self.http_referer {
            builder = builder.header("HTTP-Referer", header_value(value)?);
        }
        if let Some(value) = &self.x_title {
            builder = builder.header("X-Title", header_value(value)?);
        }
        if let Some(value) = &self.x_openrouter_title {
            builder = builder.header("X-OpenRouter-Title", header_value(value)?);
        }
        if let Some(value) = &self.x_openrouter_categories {
            builder = builder.header("X-OpenRouter-Categories", header_value(value)?);
        }
        if let Some(value) = &self.x_openrouter_metadata {
            builder = builder.header("X-OpenRouter-Metadata", header_value(value)?);
        }
        if let Some(value) = &self.session_id {
            builder = builder.header("X-Session-Id", header_value(value)?);
        }
        Ok(builder)
    }

    #[cfg(feature = "blocking")]
    pub(crate) fn apply_blocking(
        &self,
        mut builder: reqwest::blocking::RequestBuilder,
    ) -> Result<reqwest::blocking::RequestBuilder, OpenRouterError> {
        for (name, value) in &self.extra_headers {
            if has_typed_header(self, name) {
                continue;
            }
            builder = builder.header(header_name(name)?, header_value(value)?);
        }
        if let Some(value) = &self.http_referer {
            builder = builder.header("HTTP-Referer", header_value(value)?);
        }
        if let Some(value) = &self.x_title {
            builder = builder.header("X-Title", header_value(value)?);
        }
        if let Some(value) = &self.x_openrouter_title {
            builder = builder.header("X-OpenRouter-Title", header_value(value)?);
        }
        if let Some(value) = &self.x_openrouter_categories {
            builder = builder.header("X-OpenRouter-Categories", header_value(value)?);
        }
        if let Some(value) = &self.x_openrouter_metadata {
            builder = builder.header("X-OpenRouter-Metadata", header_value(value)?);
        }
        if let Some(value) = &self.session_id {
            builder = builder.header("X-Session-Id", header_value(value)?);
        }
        Ok(builder)
    }
}

fn header_name(value: &str) -> Result<HeaderName, OpenRouterError> {
    HeaderName::from_bytes(value.as_bytes())
        .map_err(|e| OpenRouterError::InvalidHeader(e.to_string()))
}

fn has_typed_header(options: &RequestOptions, name: &str) -> bool {
    match name.to_ascii_lowercase().as_str() {
        "http-referer" => options.http_referer.is_some(),
        "x-title" => options.x_title.is_some(),
        "x-openrouter-title" => options.x_openrouter_title.is_some(),
        "x-openrouter-categories" => options.x_openrouter_categories.is_some(),
        "x-openrouter-metadata" => options.x_openrouter_metadata.is_some(),
        "x-session-id" => options.session_id.is_some(),
        _ => false,
    }
}

fn header_value(value: &str) -> Result<HeaderValue, OpenRouterError> {
    HeaderValue::from_str(value).map_err(|e| OpenRouterError::InvalidHeader(e.to_string()))
}
