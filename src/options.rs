use reqwest::header::{HeaderName, HeaderValue};

use crate::OpenRouterError;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestOptions {
    pub http_referer: Option<String>,
    pub x_title: Option<String>,
    pub session_id: Option<String>,
    pub extra_headers: Vec<(String, String)>,
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

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra_headers.push((name.into(), value.into()));
        self
    }

    #[cfg(feature = "async")]
    pub(crate) fn apply_async(
        &self,
        mut builder: reqwest::RequestBuilder,
    ) -> Result<reqwest::RequestBuilder, OpenRouterError> {
        if let Some(value) = &self.http_referer {
            builder = builder.header("HTTP-Referer", header_value(value)?);
        }
        if let Some(value) = &self.x_title {
            builder = builder.header("X-Title", header_value(value)?);
        }
        if let Some(value) = &self.session_id {
            builder = builder.header("X-Session-Id", header_value(value)?);
        }
        for (name, value) in &self.extra_headers {
            builder = builder.header(header_name(name)?, header_value(value)?);
        }
        Ok(builder)
    }

    #[cfg(feature = "blocking")]
    pub(crate) fn apply_blocking(
        &self,
        mut builder: reqwest::blocking::RequestBuilder,
    ) -> Result<reqwest::blocking::RequestBuilder, OpenRouterError> {
        if let Some(value) = &self.http_referer {
            builder = builder.header("HTTP-Referer", header_value(value)?);
        }
        if let Some(value) = &self.x_title {
            builder = builder.header("X-Title", header_value(value)?);
        }
        if let Some(value) = &self.session_id {
            builder = builder.header("X-Session-Id", header_value(value)?);
        }
        for (name, value) in &self.extra_headers {
            builder = builder.header(header_name(name)?, header_value(value)?);
        }
        Ok(builder)
    }
}

fn header_name(value: &str) -> Result<HeaderName, OpenRouterError> {
    HeaderName::from_bytes(value.as_bytes())
        .map_err(|e| OpenRouterError::InvalidHeader(e.to_string()))
}

fn header_value(value: &str) -> Result<HeaderValue, OpenRouterError> {
    HeaderValue::from_str(value).map_err(|e| OpenRouterError::InvalidHeader(e.to_string()))
}
