use reqwest::{Url, blocking::multipart};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::auth::AuthRequirement;
use crate::client_routes::{dynamic_route_methods, static_route_methods};
use crate::error::{parse_api_error, reqwest_error_message};
use crate::observability::RequestTrace;
use crate::retry::{RetryPolicy, retry_after};
use crate::routes::{HttpMethod, MultipartFile, RawJsonRequest, RawMultipartRequest};
use crate::streaming::BlockingSseStream;
use crate::transport::{
    endpoint_url_from_base, normalize_base_url, normalize_unchecked_base_url, path_segment,
    with_query,
};
use crate::types::*;
use crate::{ApiKey, OpenRouterError, RequestAuth, RequestOptions};

pub struct BlockingOpenRouterClient {
    http: reqwest::blocking::Client,
    base_url: Url,
    api_key: Option<ApiKey>,
    retry_policy: RetryPolicy,
}

#[derive(Debug, Clone)]
pub struct BlockingOpenRouterClientBuilder {
    http: reqwest::blocking::Client,
    base_url: String,
    api_key: Option<ApiKey>,
    retry_policy: RetryPolicy,
    unchecked_base_url: bool,
}

impl BlockingOpenRouterClientBuilder {
    pub fn new() -> Self {
        Self {
            http: reqwest::blocking::Client::new(),
            base_url: crate::transport::DEFAULT_BASE_URL.to_owned(),
            api_key: None,
            retry_policy: RetryPolicy::default(),
            unchecked_base_url: false,
        }
    }

    pub fn http(mut self, http: reqwest::blocking::Client) -> Self {
        self.http = http;
        self
    }
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }
    pub fn api_key(mut self, api_key: impl Into<ApiKey>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }
    pub fn retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }
    pub fn unchecked_base_url(mut self, unchecked: bool) -> Self {
        self.unchecked_base_url = unchecked;
        self
    }

    pub fn build(self) -> Result<BlockingOpenRouterClient, OpenRouterError> {
        let mut client = if self.unchecked_base_url {
            BlockingOpenRouterClient::try_new_unchecked_base_url(self.http, self.base_url)?
        } else {
            BlockingOpenRouterClient::try_new(self.http, self.base_url)?
        };
        client.api_key = self.api_key;
        client.retry_policy = self.retry_policy;
        Ok(client)
    }
}

impl Default for BlockingOpenRouterClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockingOpenRouterClient {
    pub fn new(http: reqwest::blocking::Client, base_url: impl Into<String>) -> Self {
        Self::try_new(http, base_url).expect("invalid OpenRouter base URL")
    }

    pub fn try_new(
        http: reqwest::blocking::Client,
        base_url: impl Into<String>,
    ) -> Result<Self, OpenRouterError> {
        Self::from_normalized_base_url(http, normalize_base_url(base_url.into()), None)
    }

    pub fn new_with_api_key(
        http: reqwest::blocking::Client,
        base_url: impl Into<String>,
        api_key: impl Into<ApiKey>,
    ) -> Self {
        Self::try_new_with_api_key(http, base_url, api_key).expect("invalid OpenRouter base URL")
    }

    pub fn try_new_with_api_key(
        http: reqwest::blocking::Client,
        base_url: impl Into<String>,
        api_key: impl Into<ApiKey>,
    ) -> Result<Self, OpenRouterError> {
        Self::from_normalized_base_url(
            http,
            normalize_base_url(base_url.into()),
            Some(api_key.into()),
        )
    }

    pub fn try_new_unchecked_base_url(
        http: reqwest::blocking::Client,
        base_url: impl Into<String>,
    ) -> Result<Self, OpenRouterError> {
        Self::from_normalized_base_url(http, normalize_unchecked_base_url(base_url.into()), None)
    }

    pub fn try_new_unchecked_base_url_with_api_key(
        http: reqwest::blocking::Client,
        base_url: impl Into<String>,
        api_key: impl Into<ApiKey>,
    ) -> Result<Self, OpenRouterError> {
        Self::from_normalized_base_url(
            http,
            normalize_unchecked_base_url(base_url.into()),
            Some(api_key.into()),
        )
    }

    fn from_normalized_base_url(
        http: reqwest::blocking::Client,
        base_url: Result<Url, String>,
        api_key: Option<ApiKey>,
    ) -> Result<Self, OpenRouterError> {
        Ok(Self {
            http,
            base_url: base_url.map_err(OpenRouterError::InvalidBaseUrl)?,
            api_key,
            retry_policy: RetryPolicy::default(),
        })
    }

    pub fn http(&self) -> &reqwest::blocking::Client {
        &self.http
    }

    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    pub fn api_key(&self) -> Option<&ApiKey> {
        self.api_key.as_ref()
    }

    pub fn builder() -> BlockingOpenRouterClientBuilder {
        BlockingOpenRouterClientBuilder::new()
    }

    pub fn try_default() -> Result<Self, OpenRouterError> {
        Self::builder().build()
    }

    pub fn try_default_with_api_key(api_key: impl Into<ApiKey>) -> Result<Self, OpenRouterError> {
        Self::builder().api_key(api_key).build()
    }

    pub fn try_from_env() -> Result<Self, OpenRouterError> {
        let key =
            std::env::var("OPENROUTER_API_KEY").map_err(|_| OpenRouterError::MissingApiKey)?;
        Self::try_default_with_api_key(key)
    }

    pub fn retry_policy(&self) -> &RetryPolicy {
        &self.retry_policy
    }

    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    pub fn with_api_key(mut self, api_key: impl Into<ApiKey>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    pub fn without_api_key(mut self) -> Self {
        self.api_key = None;
        self
    }

    pub fn raw_json(&self, request: RawJsonRequest) -> Result<Value, OpenRouterError> {
        self.request_json_value(
            request.method,
            &request.path,
            AuthRequirement::Default,
            &request.query,
            request.body.as_ref(),
            &request.options,
        )
    }

    pub fn raw_binary(&self, request: RawJsonRequest) -> Result<BinaryResponse, OpenRouterError> {
        self.request_binary(
            request.method,
            &request.path,
            AuthRequirement::Default,
            &request.query,
            request.body.as_ref(),
            &request.options,
        )
    }

    pub fn raw_multipart(&self, request: RawMultipartRequest) -> Result<Value, OpenRouterError> {
        self.request_multipart_value(
            request.method,
            &request.path,
            AuthRequirement::Default,
            &request.query,
            request.files,
            request.fields,
            &request.options,
        )
    }

    fn request_builder(
        &self,
        method: HttpMethod,
        path: &str,
        auth: AuthRequirement,
        query: &[(String, String)],
        options: &RequestOptions,
    ) -> Result<(reqwest::blocking::RequestBuilder, bool), OpenRouterError> {
        let url = with_query(endpoint_url_from_base(&self.base_url, path)?, query);
        let mut builder = self.http.request(method.into(), url);
        let api_key = self.resolve_api_key(auth, options)?;
        if let Some(api_key) = api_key {
            builder = builder.bearer_auth(api_key);
        }
        Ok((options.apply_blocking(builder)?, api_key.is_some()))
    }

    fn resolve_api_key<'a>(
        &'a self,
        auth: AuthRequirement,
        options: &'a RequestOptions,
    ) -> Result<Option<&'a str>, OpenRouterError> {
        match &options.auth {
            RequestAuth::ApiKey(api_key) => Ok(Some(api_key.expose_secret())),
            RequestAuth::NoAuth => match auth {
                AuthRequirement::Required => Err(OpenRouterError::MissingApiKey),
                AuthRequirement::Optional | AuthRequirement::Default => Ok(None),
            },
            RequestAuth::Default => match auth {
                AuthRequirement::Required => self
                    .api_key
                    .as_ref()
                    .map(ApiKey::expose_secret)
                    .ok_or(OpenRouterError::MissingApiKey)
                    .map(Some),
                AuthRequirement::Optional => Ok(None),
                AuthRequirement::Default => Ok(self.api_key.as_ref().map(ApiKey::expose_secret)),
            },
        }
    }

    fn request_json_no_body<T: DeserializeOwned>(
        &self,
        method: HttpMethod,
        path: &str,
        auth: AuthRequirement,
        query: &[(String, String)],
        options: &RequestOptions,
    ) -> Result<T, OpenRouterError> {
        let mut attempt = 0;
        loop {
            let (builder, authenticated) =
                self.request_builder(method, path, auth, query, options)?;
            let trace = RequestTrace::start(method, path, query, authenticated);
            let resp = match builder.send() {
                Ok(resp) => resp,
                Err(e) => {
                    trace.transport_error(&e);
                    if self.retry_policy.should_retry_transport(method, attempt) {
                        std::thread::sleep(self.retry_policy.backoff(attempt, None));
                        attempt += 1;
                        continue;
                    }
                    return Err(OpenRouterError::Transport(reqwest_error_message(&e)));
                }
            };
            trace.response(resp.status(), resp.headers());
            if self
                .retry_policy
                .should_retry_status(method, attempt, resp.status().as_u16())
            {
                let delay = self
                    .retry_policy
                    .backoff(attempt, retry_after(resp.headers()));
                std::thread::sleep(delay);
                attempt += 1;
                continue;
            }
            return parse_json_response(resp);
        }
    }

    fn request_json_body<B: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        method: HttpMethod,
        path: &str,
        auth: AuthRequirement,
        query: &[(String, String)],
        body: &B,
        options: &RequestOptions,
    ) -> Result<T, OpenRouterError> {
        let mut attempt = 0;
        loop {
            let (builder, authenticated) =
                self.request_builder(method, path, auth, query, options)?;
            let builder = builder.json(body);
            let trace = RequestTrace::start(method, path, query, authenticated);
            let resp = match builder.send() {
                Ok(resp) => resp,
                Err(e) => {
                    trace.transport_error(&e);
                    if self.retry_policy.should_retry_transport(method, attempt) {
                        std::thread::sleep(self.retry_policy.backoff(attempt, None));
                        attempt += 1;
                        continue;
                    }
                    return Err(OpenRouterError::Transport(reqwest_error_message(&e)));
                }
            };
            trace.response(resp.status(), resp.headers());
            if self
                .retry_policy
                .should_retry_status(method, attempt, resp.status().as_u16())
            {
                let delay = self
                    .retry_policy
                    .backoff(attempt, retry_after(resp.headers()));
                std::thread::sleep(delay);
                attempt += 1;
                continue;
            }
            return parse_json_response(resp);
        }
    }

    fn request_json_value(
        &self,
        method: HttpMethod,
        path: &str,
        auth: AuthRequirement,
        query: &[(String, String)],
        body: Option<&Value>,
        options: &RequestOptions,
    ) -> Result<Value, OpenRouterError> {
        match body {
            Some(body) => self.request_json_body(method, path, auth, query, body, options),
            None => self.request_json_no_body(method, path, auth, query, options),
        }
    }

    fn request_binary(
        &self,
        method: HttpMethod,
        path: &str,
        auth: AuthRequirement,
        query: &[(String, String)],
        body: Option<&Value>,
        options: &RequestOptions,
    ) -> Result<BinaryResponse, OpenRouterError> {
        let mut attempt = 0;
        loop {
            let (mut builder, authenticated) =
                self.request_builder(method, path, auth, query, options)?;
            if let Some(body) = body {
                builder = builder.json(body);
            }
            let trace = RequestTrace::start(method, path, query, authenticated);
            let resp = match builder.send() {
                Ok(resp) => resp,
                Err(e) => {
                    trace.transport_error(&e);
                    if self.retry_policy.should_retry_transport(method, attempt) {
                        std::thread::sleep(self.retry_policy.backoff(attempt, None));
                        attempt += 1;
                        continue;
                    }
                    return Err(OpenRouterError::Transport(reqwest_error_message(&e)));
                }
            };
            trace.response(resp.status(), resp.headers());
            if self
                .retry_policy
                .should_retry_status(method, attempt, resp.status().as_u16())
            {
                let delay = self
                    .retry_policy
                    .backoff(attempt, retry_after(resp.headers()));
                std::thread::sleep(delay);
                attempt += 1;
                continue;
            }
            return parse_binary_response(resp);
        }
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "transport helpers keep the HTTP request pieces explicit"
    )]
    fn request_multipart_value(
        &self,
        method: HttpMethod,
        path: &str,
        auth: AuthRequirement,
        query: &[(String, String)],
        files: Vec<MultipartFile>,
        fields: Vec<(String, String)>,
        options: &RequestOptions,
    ) -> Result<Value, OpenRouterError> {
        let mut attempt = 0;
        loop {
            let form = multipart_form(files.clone(), fields.clone())?;
            let (builder, authenticated) =
                self.request_builder(method, path, auth, query, options)?;
            let builder = builder.multipart(form);
            let trace = RequestTrace::start(method, path, query, authenticated);
            let resp = match builder.send() {
                Ok(resp) => resp,
                Err(e) => {
                    trace.transport_error(&e);
                    if self.retry_policy.should_retry_transport(method, attempt) {
                        std::thread::sleep(self.retry_policy.backoff(attempt, None));
                        attempt += 1;
                        continue;
                    }
                    return Err(OpenRouterError::Transport(reqwest_error_message(&e)));
                }
            };
            trace.response(resp.status(), resp.headers());
            if self
                .retry_policy
                .should_retry_status(method, attempt, resp.status().as_u16())
            {
                let delay = self
                    .retry_policy
                    .backoff(attempt, retry_after(resp.headers()));
                std::thread::sleep(delay);
                attempt += 1;
                continue;
            }
            return parse_json_response(resp);
        }
    }

    fn stream_json_body<B: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        path: &str,
        auth: AuthRequirement,
        body: &B,
        options: &RequestOptions,
    ) -> Result<BlockingSseStream<T>, OpenRouterError> {
        let mut attempt = 0;
        loop {
            let (builder, authenticated) =
                self.request_builder(HttpMethod::Post, path, auth, &[], options)?;
            let builder = builder.json(body);
            let trace = RequestTrace::start(HttpMethod::Post, path, &[], authenticated);
            let resp = match builder.send() {
                Ok(resp) => resp,
                Err(e) => {
                    trace.transport_error(&e);
                    if self
                        .retry_policy
                        .should_retry_transport(HttpMethod::Post, attempt)
                    {
                        std::thread::sleep(self.retry_policy.backoff(attempt, None));
                        attempt += 1;
                        continue;
                    }
                    return Err(OpenRouterError::Transport(reqwest_error_message(&e)));
                }
            };
            trace.response(resp.status(), resp.headers());
            let status = resp.status();
            if self
                .retry_policy
                .should_retry_status(HttpMethod::Post, attempt, status.as_u16())
            {
                let delay = self
                    .retry_policy
                    .backoff(attempt, retry_after(resp.headers()));
                std::thread::sleep(delay);
                attempt += 1;
                continue;
            }
            if !status.is_success() {
                let headers = resp.headers().clone();
                let body = resp.text().unwrap_or_default();
                return Err(parse_api_error(status, &headers, body));
            }
            return Ok(BlockingSseStream::new(resp));
        }
    }

    pub fn create_chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, OpenRouterError> {
        self.create_chat_completion_with_options(request, RequestOptions::default())
    }

    pub fn create_chat_completion_with_options(
        &self,
        request: ChatCompletionRequest,
        options: RequestOptions,
    ) -> Result<ChatCompletionResponse, OpenRouterError> {
        self.request_json_body(
            HttpMethod::Post,
            "chat/completions",
            AuthRequirement::Required,
            &[],
            &request,
            &options,
        )
    }

    pub fn stream_chat_completion(
        &self,
        mut request: ChatCompletionRequest,
    ) -> Result<BlockingSseStream<ChatStreamChunk>, OpenRouterError> {
        request.stream = Some(true);
        self.stream_chat_completion_with_options(request, RequestOptions::default())
    }

    pub fn stream_chat_completion_with_options(
        &self,
        mut request: ChatCompletionRequest,
        options: RequestOptions,
    ) -> Result<BlockingSseStream<ChatStreamChunk>, OpenRouterError> {
        request.stream = Some(true);
        self.stream_json_body(
            "chat/completions",
            AuthRequirement::Required,
            &request,
            &options,
        )
    }

    pub fn create_response(
        &self,
        request: ResponsesRequest,
    ) -> Result<ResponsesResponse, OpenRouterError> {
        self.create_response_with_options(request, RequestOptions::default())
    }

    pub fn create_response_with_options(
        &self,
        request: ResponsesRequest,
        options: RequestOptions,
    ) -> Result<ResponsesResponse, OpenRouterError> {
        self.request_json_body(
            HttpMethod::Post,
            "responses",
            AuthRequirement::Required,
            &[],
            &request,
            &options,
        )
    }

    pub fn stream_response(
        &self,
        mut request: ResponsesRequest,
    ) -> Result<BlockingSseStream<StreamedResponsesEvent>, OpenRouterError> {
        request.stream = Some(true);
        self.stream_response_with_options(request, RequestOptions::default())
    }

    pub fn stream_response_with_options(
        &self,
        mut request: ResponsesRequest,
        options: RequestOptions,
    ) -> Result<BlockingSseStream<StreamedResponsesEvent>, OpenRouterError> {
        request.stream = Some(true);
        self.stream_json_body("responses", AuthRequirement::Required, &request, &options)
    }

    pub fn create_message(
        &self,
        request: MessagesRequest,
    ) -> Result<MessagesResponse, OpenRouterError> {
        self.create_message_with_options(request, RequestOptions::default())
    }

    pub fn create_message_with_options(
        &self,
        request: MessagesRequest,
        options: RequestOptions,
    ) -> Result<MessagesResponse, OpenRouterError> {
        self.request_json_body(
            HttpMethod::Post,
            "messages",
            AuthRequirement::Required,
            &[],
            &request,
            &options,
        )
    }

    pub fn stream_message(
        &self,
        mut request: MessagesRequest,
    ) -> Result<BlockingSseStream<MessagesStreamEvent>, OpenRouterError> {
        request.stream = Some(true);
        self.stream_message_with_options(request, RequestOptions::default())
    }

    pub fn stream_message_with_options(
        &self,
        mut request: MessagesRequest,
        options: RequestOptions,
    ) -> Result<BlockingSseStream<MessagesStreamEvent>, OpenRouterError> {
        request.stream = Some(true);
        self.stream_json_body("messages", AuthRequirement::Required, &request, &options)
    }

    pub fn generation_cost(&self, generation_id: &str) -> Result<Option<f64>, OpenRouterError> {
        match self.get_generation(generation_id) {
            Ok(generation) => Ok(generation.total_cost()),
            Err(err) if is_not_found(&err) => Ok(None),
            Err(err) => Err(err),
        }
    }
}

macro_rules! blocking_get_public {
    ($name:ident, $with:ident, $path:literal, $resp:ty) => {
        pub fn $name<Q: crate::transport::IntoQueryParams>(
            &self,
            query: Q,
        ) -> Result<$resp, OpenRouterError> {
            self.$with(query, RequestOptions::default())
        }

        pub fn $with<Q: crate::transport::IntoQueryParams>(
            &self,
            query: Q,
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            let query = query.into_query_params();
            self.request_json_no_body(
                HttpMethod::Get,
                $path,
                AuthRequirement::Optional,
                &query,
                &options,
            )
        }
    };
}

macro_rules! blocking_get_auth {
    ($name:ident, $with:ident, $path:literal, $resp:ty) => {
        pub fn $name<Q: crate::transport::IntoQueryParams>(
            &self,
            query: Q,
        ) -> Result<$resp, OpenRouterError> {
            self.$with(query, RequestOptions::default())
        }

        pub fn $with<Q: crate::transport::IntoQueryParams>(
            &self,
            query: Q,
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            let query = query.into_query_params();
            self.request_json_no_body(
                HttpMethod::Get,
                $path,
                AuthRequirement::Required,
                &query,
                &options,
            )
        }
    };
}

macro_rules! blocking_post_auth {
    ($name:ident, $with:ident, $path:literal, $req:ty, $resp:ty) => {
        pub fn $name(&self, request: $req) -> Result<$resp, OpenRouterError> {
            self.$with(request, RequestOptions::default())
        }

        pub fn $with(
            &self,
            request: $req,
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            self.request_json_body(
                HttpMethod::Post,
                $path,
                AuthRequirement::Required,
                &[],
                &request,
                &options,
            )
        }
    };
}

impl BlockingOpenRouterClient {
    static_route_methods!(blocking_get_auth, blocking_get_public, blocking_post_auth);

    pub fn exchange_auth_code_for_api_key(
        &self,
        request: AuthKeyExchangeRequest,
    ) -> Result<AuthKeyExchangeResponse, OpenRouterError> {
        self.exchange_auth_code_for_api_key_with_options(request, RequestOptions::default())
    }

    pub fn exchange_auth_code_for_api_key_with_options(
        &self,
        request: AuthKeyExchangeRequest,
        options: RequestOptions,
    ) -> Result<AuthKeyExchangeResponse, OpenRouterError> {
        self.request_json_body(
            HttpMethod::Post,
            "auth/keys",
            AuthRequirement::Optional,
            &[],
            &request,
            &options,
        )
    }
}

impl BlockingOpenRouterClient {
    pub fn create_audio_speech(
        &self,
        request: SpeechRequest,
    ) -> Result<BinaryResponse, OpenRouterError> {
        self.create_audio_speech_with_options(request, RequestOptions::default())
    }

    pub fn create_audio_speech_with_options(
        &self,
        request: SpeechRequest,
        options: RequestOptions,
    ) -> Result<BinaryResponse, OpenRouterError> {
        let body =
            serde_json::to_value(request).map_err(|e| OpenRouterError::Decode(e.to_string()))?;
        self.request_binary(
            HttpMethod::Post,
            "audio/speech",
            AuthRequirement::Required,
            &[],
            Some(&body),
            &options,
        )
    }

    pub fn create_audio_transcription_file(
        &self,
        request: TranscriptionFileRequest,
    ) -> Result<TranscriptionResponse, OpenRouterError> {
        self.create_audio_transcription_file_with_options(request, RequestOptions::default())
    }

    pub fn create_audio_transcription_file_with_options(
        &self,
        request: TranscriptionFileRequest,
        options: RequestOptions,
    ) -> Result<TranscriptionResponse, OpenRouterError> {
        let mut file = MultipartFile::new("file", request.bytes);
        file.file_name = request.file_name;
        file.content_type = request.content_type;

        let mut fields = vec![("model".to_owned(), request.model)];
        if let Some(language) = request.language {
            fields.push(("language".to_owned(), language));
        }
        if let Some(response_format) = request.response_format {
            fields.push(("response_format".to_owned(), response_format));
        }
        if let Some(temperature) = request.temperature {
            fields.push(("temperature".to_owned(), temperature.to_string()));
        }

        let value = self.request_multipart_value(
            HttpMethod::Post,
            "audio/transcriptions",
            AuthRequirement::Required,
            &[],
            vec![file],
            fields,
            &options,
        )?;
        serde_json::from_value(value).map_err(|e| OpenRouterError::Decode(e.to_string()))
    }

    pub fn upload_file(
        &self,
        request: FileUploadRequest,
    ) -> Result<FileUploadResponse, OpenRouterError> {
        self.upload_file_with_options(request, RequestOptions::default())
    }

    pub fn upload_file_with_options(
        &self,
        request: FileUploadRequest,
        options: RequestOptions,
    ) -> Result<FileUploadResponse, OpenRouterError> {
        let mut file = MultipartFile::new("file", request.bytes);
        file.file_name = request.file_name;
        file.content_type = request.content_type;
        let value = self.request_multipart_value(
            HttpMethod::Post,
            "files",
            AuthRequirement::Required,
            &[],
            vec![file],
            Vec::new(),
            &options,
        )?;
        serde_json::from_value(value).map_err(|e| OpenRouterError::Decode(e.to_string()))
    }

    pub fn get_generation(
        &self,
        generation_id: &str,
    ) -> Result<GenerationResponse, OpenRouterError> {
        self.get_generation_with_options(generation_id, RequestOptions::default())
    }

    pub fn get_generation_with_options(
        &self,
        generation_id: &str,
        options: RequestOptions,
    ) -> Result<GenerationResponse, OpenRouterError> {
        self.request_json_no_body(
            HttpMethod::Get,
            "generation",
            AuthRequirement::Required,
            &[("id".to_owned(), generation_id.to_owned())],
            &options,
        )
    }

    pub fn get_generation_content(
        &self,
        generation_id: &str,
    ) -> Result<GenerationContentResponse, OpenRouterError> {
        self.get_generation_content_with_options(generation_id, RequestOptions::default())
    }

    pub fn get_generation_content_with_options(
        &self,
        generation_id: &str,
        options: RequestOptions,
    ) -> Result<GenerationContentResponse, OpenRouterError> {
        self.request_json_no_body(
            HttpMethod::Get,
            "generation/content",
            AuthRequirement::Required,
            &[("id".to_owned(), generation_id.to_owned())],
            &options,
        )
    }
}

macro_rules! dyn_get_auth {
    ($name:ident, $with:ident, $resp:ty, |$($arg:ident : $typ:ty),+| $path:expr) => {
        pub fn $name<Q: crate::transport::IntoQueryParams>(
            &self,
            $($arg: $typ,)+
            query: Q,
        ) -> Result<$resp, OpenRouterError> {
            self.$with($($arg,)+ query, RequestOptions::default())
        }

        pub fn $with<Q: crate::transport::IntoQueryParams>(
            &self,
            $($arg: $typ,)+
            query: Q,
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            let path = $path;
            let query = query.into_query_params();
            self.request_json_no_body(
                HttpMethod::Get,
                &path,
                AuthRequirement::Required,
                &query,
                &options,
            )
        }
    };
}

macro_rules! dyn_get_public {
    ($name:ident, $with:ident, $resp:ty, |$($arg:ident : $typ:ty),+| $path:expr) => {
        pub fn $name<Q: crate::transport::IntoQueryParams>(
            &self,
            $($arg: $typ,)+
            query: Q,
        ) -> Result<$resp, OpenRouterError> {
            self.$with($($arg,)+ query, RequestOptions::default())
        }

        pub fn $with<Q: crate::transport::IntoQueryParams>(
            &self,
            $($arg: $typ,)+
            query: Q,
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            let path = $path;
            let query = query.into_query_params();
            self.request_json_no_body(
                HttpMethod::Get,
                &path,
                AuthRequirement::Optional,
                &query,
                &options,
            )
        }
    };
}

macro_rules! dyn_delete_auth {
    ($name:ident, $with:ident, $resp:ty, |$($arg:ident : $typ:ty),+| $path:expr) => {
        pub fn $name(&self, $($arg: $typ),+) -> Result<$resp, OpenRouterError> {
            self.$with($($arg,)+ RequestOptions::default())
        }

        pub fn $with(
            &self,
            $($arg: $typ,)+
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            let path = $path;
            self.request_json_no_body(
                HttpMethod::Delete,
                &path,
                AuthRequirement::Required,
                &[],
                &options,
            )
        }
    };
}

macro_rules! dyn_patch_auth {
    ($name:ident, $with:ident, $req:ty, $resp:ty, |$($arg:ident : $typ:ty),+| $path:expr) => {
        pub fn $name(
            &self,
            $($arg: $typ,)+
            request: $req,
        ) -> Result<$resp, OpenRouterError> {
            self.$with($($arg,)+ request, RequestOptions::default())
        }

        pub fn $with(
            &self,
            $($arg: $typ,)+
            request: $req,
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            let path = $path;
            self.request_json_body(
                HttpMethod::Patch,
                &path,
                AuthRequirement::Required,
                &[],
                &request,
                &options,
            )
        }
    };
}

macro_rules! dyn_put_auth {
    ($name:ident, $with:ident, $req:ty, $resp:ty, |$($arg:ident : $typ:ty),+| $path:expr) => {
        pub fn $name(
            &self,
            $($arg: $typ,)+
            request: $req,
        ) -> Result<$resp, OpenRouterError> {
            self.$with($($arg,)+ request, RequestOptions::default())
        }

        pub fn $with(
            &self,
            $($arg: $typ,)+
            request: $req,
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            let path = $path;
            self.request_json_body(
                HttpMethod::Put,
                &path,
                AuthRequirement::Required,
                &[],
                &request,
                &options,
            )
        }
    };
}

macro_rules! dyn_post_auth {
    ($name:ident, $with:ident, $req:ty, $resp:ty, |$($arg:ident : $typ:ty),+| $path:expr) => {
        pub fn $name(
            &self,
            $($arg: $typ,)+
            request: $req,
        ) -> Result<$resp, OpenRouterError> {
            self.$with($($arg,)+ request, RequestOptions::default())
        }

        pub fn $with(
            &self,
            $($arg: $typ,)+
            request: $req,
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            let path = $path;
            self.request_json_body(
                HttpMethod::Post,
                &path,
                AuthRequirement::Required,
                &[],
                &request,
                &options,
            )
        }
    };
}

impl BlockingOpenRouterClient {
    dynamic_route_methods!(
        dyn_get_auth,
        dyn_get_public,
        dyn_delete_auth,
        dyn_patch_auth,
        dyn_put_auth,
        dyn_post_auth
    );

    pub fn download_file_content(&self, file_id: &str) -> Result<BinaryResponse, OpenRouterError> {
        self.download_file_content_with_options(file_id, RequestOptions::default())
    }

    pub fn download_file_content_with_options(
        &self,
        file_id: &str,
        options: RequestOptions,
    ) -> Result<BinaryResponse, OpenRouterError> {
        self.request_binary(
            HttpMethod::Get,
            &format!("files/{}/content", path_segment(file_id)),
            AuthRequirement::Required,
            &[],
            None,
            &options,
        )
    }

    pub fn download_video_content(&self, job_id: &str) -> Result<BinaryResponse, OpenRouterError> {
        self.download_video_content_with_options(job_id, RequestOptions::default())
    }

    pub fn download_video_content_with_options(
        &self,
        job_id: &str,
        options: RequestOptions,
    ) -> Result<BinaryResponse, OpenRouterError> {
        self.request_binary(
            HttpMethod::Get,
            &format!("videos/{}/content", path_segment(job_id)),
            AuthRequirement::Required,
            &[],
            None,
            &options,
        )
    }
}

fn parse_json_response<T: DeserializeOwned>(
    resp: reqwest::blocking::Response,
) -> Result<T, OpenRouterError> {
    let status = resp.status();
    if !status.is_success() {
        let headers = resp.headers().clone();
        let body = resp.text().unwrap_or_default();
        return Err(parse_api_error(status, &headers, body));
    }

    resp.json()
        .map_err(|e| OpenRouterError::Decode(e.to_string()))
}

fn parse_binary_response(
    resp: reqwest::blocking::Response,
) -> Result<BinaryResponse, OpenRouterError> {
    let status = resp.status();
    let headers = resp.headers().clone();
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        return Err(parse_api_error(status, &headers, body));
    }

    let content_type = headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    let content_disposition = headers
        .get(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    let bytes = resp
        .bytes()
        .map_err(|e| OpenRouterError::Transport(reqwest_error_message(&e)))?;
    Ok(BinaryResponse {
        bytes,
        content_type,
        content_disposition,
    })
}

fn multipart_form(
    files: Vec<MultipartFile>,
    fields: Vec<(String, String)>,
) -> Result<multipart::Form, OpenRouterError> {
    let mut form = multipart::Form::new();
    for (key, value) in fields {
        form = form.text(key, value);
    }
    for file in files {
        let length = u64::try_from(file.bytes.len()).map_err(|_| {
            OpenRouterError::InvalidHeader("multipart file is too large".to_owned())
        })?;
        let mut part =
            multipart::Part::reader_with_length(std::io::Cursor::new(file.bytes), length);
        if let Some(file_name) = file.file_name {
            part = part.file_name(file_name);
        }
        if let Some(content_type) = file.content_type {
            part = part
                .mime_str(&content_type)
                .map_err(|e| OpenRouterError::InvalidHeader(e.to_string()))?;
        }
        form = form.part(file.field_name, part);
    }
    Ok(form)
}

fn is_not_found(err: &OpenRouterError) -> bool {
    matches!(err, OpenRouterError::Api(api) if api.status == 404)
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc;
    use std::thread;

    use crate::streaming::SseMessage;
    use crate::{
        AuthKeyExchangeRequest, BlockingOpenRouterClient, ChatCompletionRequest, ChatMessage,
        HttpMethod, OpenRouterError, PaginationQuery, RawJsonRequest, RequestOptions,
    };

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

    fn serve_once(
        status: &'static str,
        content_type: &'static str,
        body: impl Into<String>,
    ) -> (String, mpsc::Receiver<RecordedRequest>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let body = body.into();
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            tx.send(request).unwrap();

            let response = format!(
                "HTTP/1.1 {status}\r\n\
                 content-type: {content_type}\r\n\
                 content-length: {}\r\n\
                 connection: close\r\n\
                 \r\n\
                 {body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        });

        (format!("http://{addr}"), rx)
    }

    fn read_request(stream: &mut TcpStream) -> RecordedRequest {
        let mut buf = Vec::new();
        let mut header_end = None;
        let mut content_length = 0;

        loop {
            let mut chunk = [0_u8; 1024];
            let n = stream.read(&mut chunk).unwrap();
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);

            if header_end.is_none() {
                if let Some(end) = buf.windows(4).position(|window| window == b"\r\n\r\n") {
                    header_end = Some(end);
                    content_length = String::from_utf8_lossy(&buf[..end])
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
                        .unwrap_or(0);
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

        RecordedRequest {
            method,
            path,
            headers,
            body,
        }
    }

    #[test]
    fn blocking_chat_completion_posts_expected_json() {
        let (base_url, request) = serve_once(
            "200 OK",
            "application/json",
            r#"{"id":"gen-123","choices":[{"message":{"role":"assistant","content":"ok"}}]}"#,
        );
        let client = BlockingOpenRouterClient::try_new_unchecked_base_url_with_api_key(
            reqwest::blocking::Client::new(),
            base_url,
            "sk-test",
        )
        .unwrap();

        let response = client
            .create_chat_completion(ChatCompletionRequest::new(
                "openai/gpt-4o-mini",
                vec![ChatMessage::user("Say hi.")],
            ))
            .unwrap();

        assert_eq!(response.id.as_deref(), Some("gen-123"));
        let recorded = request.recv().unwrap();
        assert_eq!(recorded.method, "POST");
        assert_eq!(recorded.path, "/chat/completions");
        assert_eq!(recorded.header("authorization"), Some("Bearer sk-test"));
        assert!(recorded.body.contains("openai/gpt-4o-mini"));
    }

    #[test]
    fn blocking_streaming_iterator_parses_events() {
        let (base_url, _request) = serve_once(
            "200 OK",
            "text/event-stream",
            "data: {\"id\":\"chunk-1\"}\n\ndata: [DONE]\n\n",
        );
        let client = BlockingOpenRouterClient::try_new_unchecked_base_url_with_api_key(
            reqwest::blocking::Client::new(),
            base_url,
            "sk-test",
        )
        .unwrap();

        let mut stream = client
            .stream_chat_completion(ChatCompletionRequest::new(
                "openai/gpt-4o-mini",
                vec![ChatMessage::user("Say hi.")],
            ))
            .unwrap();

        match stream.next().unwrap().unwrap() {
            SseMessage::Data(chunk) => assert_eq!(chunk.id.as_deref(), Some("chunk-1")),
            other => panic!("unexpected event: {other:?}"),
        }
        assert_eq!(stream.next().unwrap().unwrap(), SseMessage::Done);
    }

    #[test]
    fn blocking_auth_code_exchange_does_not_send_authorization() {
        let (base_url, request) = serve_once(
            "200 OK",
            "application/json",
            r#"{"key":"sk-new","user_id":null}"#,
        );
        let client = BlockingOpenRouterClient::try_new_unchecked_base_url_with_api_key(
            reqwest::blocking::Client::new(),
            base_url,
            "sk-should-not-send",
        )
        .unwrap();

        let response = client
            .exchange_auth_code_for_api_key_with_options(
                AuthKeyExchangeRequest::new().with_field("code", "abc"),
                RequestOptions::new().without_auth(),
            )
            .unwrap();

        assert_eq!(response.extra["key"], "sk-new");
        let recorded = request.recv().unwrap();
        assert_eq!(recorded.method, "POST");
        assert_eq!(recorded.path, "/auth/keys");
        assert_eq!(recorded.header("authorization"), None);
    }

    #[test]
    fn blocking_generation_cost_returns_none_for_not_yet_queryable_generation() {
        let (base_url, request) = serve_once("404 Not Found", "application/json", "{}");
        let client = BlockingOpenRouterClient::try_new_unchecked_base_url_with_api_key(
            reqwest::blocking::Client::new(),
            base_url,
            "sk-cost",
        )
        .unwrap();

        let cost = client.generation_cost("gen-789").unwrap();

        assert_eq!(cost, None);
        let recorded = request.recv().unwrap();
        assert_eq!(recorded.method, "GET");
        assert_eq!(recorded.path, "/generation?id=gen-789");
        assert_eq!(recorded.header("authorization"), Some("Bearer sk-cost"));
    }

    #[test]
    fn blocking_missing_api_key_errors_before_send() {
        let client = BlockingOpenRouterClient::try_new_unchecked_base_url(
            reqwest::blocking::Client::new(),
            "http://127.0.0.1:9",
        )
        .unwrap();

        let err = client
            .create_chat_completion(ChatCompletionRequest::new(
                "openai/gpt-4o-mini",
                vec![ChatMessage::user("Say hi.")],
            ))
            .unwrap_err();

        assert!(matches!(err, OpenRouterError::MissingApiKey));
    }

    #[test]
    fn blocking_list_workspace_members_sends_typed_pagination_query() {
        let (base_url, request) = serve_once(
            "200 OK",
            "application/json",
            r#"{"data":[],"total_count":0}"#,
        );
        let client = BlockingOpenRouterClient::try_new_unchecked_base_url_with_api_key(
            reqwest::blocking::Client::new(),
            base_url,
            "sk-workspace",
        )
        .unwrap();

        let mut query = PaginationQuery::new();
        query.offset = Some(1);
        query.limit = Some(2);

        let response = client.list_workspace_members("production", query).unwrap();

        assert_eq!(response.total_count, Some(0));
        let recorded = request.recv().unwrap();
        assert_eq!(
            recorded.path,
            "/workspaces/production/members?offset=1&limit=2"
        );
        assert_eq!(
            recorded.header("authorization"),
            Some("Bearer sk-workspace")
        );
    }

    #[test]
    fn raw_absolute_paths_are_rejected_before_send() {
        let client = BlockingOpenRouterClient::try_new_unchecked_base_url(
            reqwest::blocking::Client::new(),
            "http://127.0.0.1:9",
        )
        .unwrap();

        let err = client
            .raw_json(RawJsonRequest::new(
                HttpMethod::Get,
                "https://user:pass@example.test/secret",
            ))
            .unwrap_err();

        assert!(matches!(err, OpenRouterError::InvalidBaseUrl(_)));
    }

    #[test]
    #[ignore = "requires OPENROUTER_API_KEY and live OpenRouter access"]
    fn live_smoke_get_current_key() {
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .expect("OPENROUTER_API_KEY must be set for live smoke tests");
        let client = BlockingOpenRouterClient::try_new_with_api_key(
            reqwest::blocking::Client::new(),
            crate::DEFAULT_BASE_URL,
            api_key,
        )
        .unwrap();

        let _ = client
            .get_current_key(())
            .expect("live get_current_key request should succeed");
    }
}
