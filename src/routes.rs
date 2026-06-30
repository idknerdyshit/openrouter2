use bytes::Bytes;
use serde_json::Value;

use crate::RequestOptions;
use crate::transport::QueryParams;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl From<HttpMethod> for reqwest::Method {
    fn from(value: HttpMethod) -> Self {
        match value {
            HttpMethod::Get => reqwest::Method::GET,
            HttpMethod::Post => reqwest::Method::POST,
            HttpMethod::Put => reqwest::Method::PUT,
            HttpMethod::Patch => reqwest::Method::PATCH,
            HttpMethod::Delete => reqwest::Method::DELETE,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RawJsonRequest {
    pub method: HttpMethod,
    pub path: String,
    pub query: QueryParams,
    pub body: Option<Value>,
    pub options: RequestOptions,
}

impl RawJsonRequest {
    pub fn new(method: HttpMethod, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            query: Vec::new(),
            body: None,
            options: RequestOptions::default(),
        }
    }

    pub fn with_query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query.push((key.into(), value.into()));
        self
    }

    pub fn with_body(mut self, body: Value) -> Self {
        self.body = Some(body);
        self
    }

    pub fn with_options(mut self, options: RequestOptions) -> Self {
        self.options = options;
        self
    }
}

#[derive(Debug, Clone)]
pub struct MultipartFile {
    pub field_name: String,
    pub file_name: Option<String>,
    pub content_type: Option<String>,
    pub bytes: Bytes,
}

impl MultipartFile {
    pub fn new(field_name: impl Into<String>, bytes: impl Into<Bytes>) -> Self {
        Self {
            field_name: field_name.into(),
            file_name: None,
            content_type: None,
            bytes: bytes.into(),
        }
    }

    pub fn with_file_name(mut self, file_name: impl Into<String>) -> Self {
        self.file_name = Some(file_name.into());
        self
    }

    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct RawMultipartRequest {
    pub method: HttpMethod,
    pub path: String,
    pub query: QueryParams,
    pub files: Vec<MultipartFile>,
    pub fields: Vec<(String, String)>,
    pub options: RequestOptions,
}

impl RawMultipartRequest {
    pub fn new(method: HttpMethod, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            query: Vec::new(),
            files: Vec::new(),
            fields: Vec::new(),
            options: RequestOptions::default(),
        }
    }

    pub fn with_file(mut self, file: MultipartFile) -> Self {
        self.files.push(file);
        self
    }

    pub fn with_query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query.push((key.into(), value.into()));
        self
    }

    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.push((key.into(), value.into()));
        self
    }

    pub fn with_options(mut self, options: RequestOptions) -> Self {
        self.options = options;
        self
    }
}
