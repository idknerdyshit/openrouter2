use reqwest::{Url, multipart};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::auth::AuthRequirement;
use crate::client_routes::{dynamic_route_methods, static_route_methods};
use crate::error::{parse_api_error, reqwest_error_message};
use crate::observability::RequestTrace;
use crate::routes::{HttpMethod, MultipartFile, RawJsonRequest, RawMultipartRequest};
use crate::streaming::{AsyncSseStream, decode_async_sse};
use crate::transport::{
    endpoint_url_from_base, normalize_base_url, normalize_unchecked_base_url, path_segment,
    with_query,
};
use crate::types::*;
use crate::{ApiKey, OpenRouterError, RequestAuth, RequestOptions};

pub struct AsyncOpenRouterClient {
    http: reqwest::Client,
    base_url: Url,
    api_key: Option<ApiKey>,
}

impl AsyncOpenRouterClient {
    pub fn new(http: reqwest::Client, base_url: impl Into<String>) -> Self {
        Self::try_new(http, base_url).expect("invalid OpenRouter base URL")
    }

    pub fn try_new(
        http: reqwest::Client,
        base_url: impl Into<String>,
    ) -> Result<Self, OpenRouterError> {
        Self::from_normalized_base_url(http, normalize_base_url(base_url.into()), None)
    }

    pub fn new_with_api_key(
        http: reqwest::Client,
        base_url: impl Into<String>,
        api_key: impl Into<ApiKey>,
    ) -> Self {
        Self::try_new_with_api_key(http, base_url, api_key).expect("invalid OpenRouter base URL")
    }

    pub fn try_new_with_api_key(
        http: reqwest::Client,
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
        http: reqwest::Client,
        base_url: impl Into<String>,
    ) -> Result<Self, OpenRouterError> {
        Self::from_normalized_base_url(http, normalize_unchecked_base_url(base_url.into()), None)
    }

    pub fn try_new_unchecked_base_url_with_api_key(
        http: reqwest::Client,
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
        http: reqwest::Client,
        base_url: Result<Url, String>,
        api_key: Option<ApiKey>,
    ) -> Result<Self, OpenRouterError> {
        Ok(Self {
            http,
            base_url: base_url.map_err(OpenRouterError::InvalidBaseUrl)?,
            api_key,
        })
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    pub fn api_key(&self) -> Option<&ApiKey> {
        self.api_key.as_ref()
    }

    pub fn with_api_key(mut self, api_key: impl Into<ApiKey>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    pub fn without_api_key(mut self) -> Self {
        self.api_key = None;
        self
    }

    pub async fn raw_json(&self, request: RawJsonRequest) -> Result<Value, OpenRouterError> {
        self.request_json_value(
            request.method,
            &request.path,
            AuthRequirement::Default,
            &request.query,
            request.body.as_ref(),
            &request.options,
        )
        .await
    }

    pub async fn raw_binary(
        &self,
        request: RawJsonRequest,
    ) -> Result<BinaryResponse, OpenRouterError> {
        self.request_binary(
            request.method,
            &request.path,
            AuthRequirement::Default,
            &request.query,
            request.body.as_ref(),
            &request.options,
        )
        .await
    }

    pub async fn raw_multipart(
        &self,
        request: RawMultipartRequest,
    ) -> Result<Value, OpenRouterError> {
        self.request_multipart_value(
            request.method,
            &request.path,
            AuthRequirement::Default,
            &request.query,
            request.files,
            request.fields,
            &request.options,
        )
        .await
    }

    fn request_builder(
        &self,
        method: HttpMethod,
        path: &str,
        auth: AuthRequirement,
        query: &[(String, String)],
        options: &RequestOptions,
    ) -> Result<(reqwest::RequestBuilder, bool), OpenRouterError> {
        let url = with_query(endpoint_url_from_base(&self.base_url, path)?, query);
        let mut builder = self.http.request(method.into(), url);
        let api_key = self.resolve_api_key(auth, options)?;
        if let Some(api_key) = api_key {
            builder = builder.bearer_auth(api_key);
        }
        Ok((options.apply_async(builder)?, api_key.is_some()))
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

    async fn request_json_no_body<T: DeserializeOwned>(
        &self,
        method: HttpMethod,
        path: &str,
        auth: AuthRequirement,
        query: &[(String, String)],
        options: &RequestOptions,
    ) -> Result<T, OpenRouterError> {
        let (builder, authenticated) = self.request_builder(method, path, auth, query, options)?;
        let trace = RequestTrace::start(method, path, query, authenticated);
        let resp = match builder.send().await {
            Ok(resp) => resp,
            Err(e) => {
                trace.transport_error(&e);
                return Err(OpenRouterError::Transport(reqwest_error_message(&e)));
            }
        };
        trace.response(resp.status(), resp.headers());
        parse_json_response(resp).await
    }

    async fn request_json_body<B: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        method: HttpMethod,
        path: &str,
        auth: AuthRequirement,
        query: &[(String, String)],
        body: &B,
        options: &RequestOptions,
    ) -> Result<T, OpenRouterError> {
        let (builder, authenticated) = self.request_builder(method, path, auth, query, options)?;
        let builder = builder.json(body);
        let trace = RequestTrace::start(method, path, query, authenticated);
        let resp = match builder.send().await {
            Ok(resp) => resp,
            Err(e) => {
                trace.transport_error(&e);
                return Err(OpenRouterError::Transport(reqwest_error_message(&e)));
            }
        };
        trace.response(resp.status(), resp.headers());
        parse_json_response(resp).await
    }

    async fn request_json_value(
        &self,
        method: HttpMethod,
        path: &str,
        auth: AuthRequirement,
        query: &[(String, String)],
        body: Option<&Value>,
        options: &RequestOptions,
    ) -> Result<Value, OpenRouterError> {
        match body {
            Some(body) => {
                self.request_json_body(method, path, auth, query, body, options)
                    .await
            }
            None => {
                self.request_json_no_body(method, path, auth, query, options)
                    .await
            }
        }
    }

    async fn request_binary(
        &self,
        method: HttpMethod,
        path: &str,
        auth: AuthRequirement,
        query: &[(String, String)],
        body: Option<&Value>,
        options: &RequestOptions,
    ) -> Result<BinaryResponse, OpenRouterError> {
        let (mut builder, authenticated) =
            self.request_builder(method, path, auth, query, options)?;
        if let Some(body) = body {
            builder = builder.json(body);
        }
        let trace = RequestTrace::start(method, path, query, authenticated);
        let resp = match builder.send().await {
            Ok(resp) => resp,
            Err(e) => {
                trace.transport_error(&e);
                return Err(OpenRouterError::Transport(reqwest_error_message(&e)));
            }
        };
        trace.response(resp.status(), resp.headers());
        parse_binary_response(resp).await
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "transport helpers keep the HTTP request pieces explicit"
    )]
    async fn request_multipart_value(
        &self,
        method: HttpMethod,
        path: &str,
        auth: AuthRequirement,
        query: &[(String, String)],
        files: Vec<MultipartFile>,
        fields: Vec<(String, String)>,
        options: &RequestOptions,
    ) -> Result<Value, OpenRouterError> {
        let form = multipart_form(files, fields)?;
        let (builder, authenticated) = self.request_builder(method, path, auth, query, options)?;
        let builder = builder.multipart(form);
        let trace = RequestTrace::start(method, path, query, authenticated);
        let resp = match builder.send().await {
            Ok(resp) => resp,
            Err(e) => {
                trace.transport_error(&e);
                return Err(OpenRouterError::Transport(reqwest_error_message(&e)));
            }
        };
        trace.response(resp.status(), resp.headers());
        parse_json_response(resp).await
    }

    async fn stream_json_body<B: Serialize + ?Sized, T: DeserializeOwned + Send + 'static>(
        &self,
        path: &str,
        auth: AuthRequirement,
        body: &B,
        options: &RequestOptions,
    ) -> Result<AsyncSseStream<T>, OpenRouterError> {
        let (builder, authenticated) =
            self.request_builder(HttpMethod::Post, path, auth, &[], options)?;
        let builder = builder.json(body);
        let trace = RequestTrace::start(HttpMethod::Post, path, &[], authenticated);
        let resp = match builder.send().await {
            Ok(resp) => resp,
            Err(e) => {
                trace.transport_error(&e);
                return Err(OpenRouterError::Transport(reqwest_error_message(&e)));
            }
        };
        trace.response(resp.status(), resp.headers());
        let status = resp.status();
        if !status.is_success() {
            let headers = resp.headers().clone();
            let body = resp.text().await.unwrap_or_default();
            return Err(parse_api_error(status, &headers, body));
        }
        Ok(decode_async_sse(resp))
    }

    pub async fn create_chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, OpenRouterError> {
        self.create_chat_completion_with_options(request, RequestOptions::default())
            .await
    }

    pub async fn create_chat_completion_with_options(
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
        .await
    }

    pub async fn stream_chat_completion(
        &self,
        mut request: ChatCompletionRequest,
    ) -> Result<AsyncSseStream<ChatStreamChunk>, OpenRouterError> {
        request.stream = Some(true);
        self.stream_chat_completion_with_options(request, RequestOptions::default())
            .await
    }

    pub async fn stream_chat_completion_with_options(
        &self,
        mut request: ChatCompletionRequest,
        options: RequestOptions,
    ) -> Result<AsyncSseStream<ChatStreamChunk>, OpenRouterError> {
        request.stream = Some(true);
        self.stream_json_body(
            "chat/completions",
            AuthRequirement::Required,
            &request,
            &options,
        )
        .await
    }

    pub async fn create_response(
        &self,
        request: ResponsesRequest,
    ) -> Result<ResponsesResponse, OpenRouterError> {
        self.create_response_with_options(request, RequestOptions::default())
            .await
    }

    pub async fn create_response_with_options(
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
        .await
    }

    pub async fn stream_response(
        &self,
        mut request: ResponsesRequest,
    ) -> Result<AsyncSseStream<StreamedResponsesEvent>, OpenRouterError> {
        request.stream = Some(true);
        self.stream_response_with_options(request, RequestOptions::default())
            .await
    }

    pub async fn stream_response_with_options(
        &self,
        mut request: ResponsesRequest,
        options: RequestOptions,
    ) -> Result<AsyncSseStream<StreamedResponsesEvent>, OpenRouterError> {
        request.stream = Some(true);
        self.stream_json_body("responses", AuthRequirement::Required, &request, &options)
            .await
    }

    pub async fn create_message(
        &self,
        request: MessagesRequest,
    ) -> Result<MessagesResponse, OpenRouterError> {
        self.create_message_with_options(request, RequestOptions::default())
            .await
    }

    pub async fn create_message_with_options(
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
        .await
    }

    pub async fn stream_message(
        &self,
        mut request: MessagesRequest,
    ) -> Result<AsyncSseStream<MessagesStreamEvent>, OpenRouterError> {
        request.stream = Some(true);
        self.stream_message_with_options(request, RequestOptions::default())
            .await
    }

    pub async fn stream_message_with_options(
        &self,
        mut request: MessagesRequest,
        options: RequestOptions,
    ) -> Result<AsyncSseStream<MessagesStreamEvent>, OpenRouterError> {
        request.stream = Some(true);
        self.stream_json_body("messages", AuthRequirement::Required, &request, &options)
            .await
    }

    pub async fn generation_cost(
        &self,
        generation_id: &str,
    ) -> Result<Option<f64>, OpenRouterError> {
        match self.get_generation(generation_id).await {
            Ok(generation) => Ok(generation.total_cost()),
            Err(err) if is_not_found(&err) => Ok(None),
            Err(err) => Err(err),
        }
    }
}

macro_rules! async_get_public {
    ($name:ident, $with:ident, $path:literal, $resp:ty) => {
        pub async fn $name<Q: crate::transport::IntoQueryParams>(
            &self,
            query: Q,
        ) -> Result<$resp, OpenRouterError> {
            self.$with(query, RequestOptions::default()).await
        }

        pub async fn $with<Q: crate::transport::IntoQueryParams>(
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
            .await
        }
    };
}

macro_rules! async_get_auth {
    ($name:ident, $with:ident, $path:literal, $resp:ty) => {
        pub async fn $name<Q: crate::transport::IntoQueryParams>(
            &self,
            query: Q,
        ) -> Result<$resp, OpenRouterError> {
            self.$with(query, RequestOptions::default()).await
        }

        pub async fn $with<Q: crate::transport::IntoQueryParams>(
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
            .await
        }
    };
}

macro_rules! async_post_auth {
    ($name:ident, $with:ident, $path:literal, $req:ty, $resp:ty) => {
        pub async fn $name(&self, request: $req) -> Result<$resp, OpenRouterError> {
            self.$with(request, RequestOptions::default()).await
        }

        pub async fn $with(
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
            .await
        }
    };
}

impl AsyncOpenRouterClient {
    static_route_methods!(async_get_auth, async_get_public, async_post_auth);

    pub async fn exchange_auth_code_for_api_key(
        &self,
        request: AuthKeyExchangeRequest,
    ) -> Result<AuthKeyExchangeResponse, OpenRouterError> {
        self.exchange_auth_code_for_api_key_with_options(request, RequestOptions::default())
            .await
    }

    pub async fn exchange_auth_code_for_api_key_with_options(
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
        .await
    }
}

impl AsyncOpenRouterClient {
    pub async fn create_audio_speech(
        &self,
        request: SpeechRequest,
    ) -> Result<BinaryResponse, OpenRouterError> {
        self.create_audio_speech_with_options(request, RequestOptions::default())
            .await
    }

    pub async fn create_audio_speech_with_options(
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
        .await
    }

    pub async fn create_audio_transcription_file(
        &self,
        request: TranscriptionFileRequest,
    ) -> Result<TranscriptionResponse, OpenRouterError> {
        self.create_audio_transcription_file_with_options(request, RequestOptions::default())
            .await
    }

    pub async fn create_audio_transcription_file_with_options(
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

        let value = self
            .request_multipart_value(
                HttpMethod::Post,
                "audio/transcriptions",
                AuthRequirement::Required,
                &[],
                vec![file],
                fields,
                &options,
            )
            .await?;
        serde_json::from_value(value).map_err(|e| OpenRouterError::Decode(e.to_string()))
    }

    pub async fn upload_file(
        &self,
        request: FileUploadRequest,
    ) -> Result<FileUploadResponse, OpenRouterError> {
        self.upload_file_with_options(request, RequestOptions::default())
            .await
    }

    pub async fn upload_file_with_options(
        &self,
        request: FileUploadRequest,
        options: RequestOptions,
    ) -> Result<FileUploadResponse, OpenRouterError> {
        let mut file = MultipartFile::new("file", request.bytes);
        file.file_name = request.file_name;
        file.content_type = request.content_type;
        let value = self
            .request_multipart_value(
                HttpMethod::Post,
                "files",
                AuthRequirement::Required,
                &[],
                vec![file],
                Vec::new(),
                &options,
            )
            .await?;
        serde_json::from_value(value).map_err(|e| OpenRouterError::Decode(e.to_string()))
    }

    pub async fn get_generation(
        &self,
        generation_id: &str,
    ) -> Result<GenerationResponse, OpenRouterError> {
        self.get_generation_with_options(generation_id, RequestOptions::default())
            .await
    }

    pub async fn get_generation_with_options(
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
        .await
    }

    pub async fn get_generation_content(
        &self,
        generation_id: &str,
    ) -> Result<GenerationContentResponse, OpenRouterError> {
        self.get_generation_content_with_options(generation_id, RequestOptions::default())
            .await
    }

    pub async fn get_generation_content_with_options(
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
        .await
    }
}

macro_rules! dyn_get_auth {
    ($name:ident, $with:ident, $resp:ty, |$($arg:ident : $typ:ty),+| $path:expr) => {
        pub async fn $name<Q: crate::transport::IntoQueryParams>(
            &self,
            $($arg: $typ,)+
            query: Q,
        ) -> Result<$resp, OpenRouterError> {
            self.$with($($arg,)+ query, RequestOptions::default()).await
        }

        pub async fn $with<Q: crate::transport::IntoQueryParams>(
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
            .await
        }
    };
}

macro_rules! dyn_get_public {
    ($name:ident, $with:ident, $resp:ty, |$($arg:ident : $typ:ty),+| $path:expr) => {
        pub async fn $name<Q: crate::transport::IntoQueryParams>(
            &self,
            $($arg: $typ,)+
            query: Q,
        ) -> Result<$resp, OpenRouterError> {
            self.$with($($arg,)+ query, RequestOptions::default()).await
        }

        pub async fn $with<Q: crate::transport::IntoQueryParams>(
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
            .await
        }
    };
}

macro_rules! dyn_delete_auth {
    ($name:ident, $with:ident, $resp:ty, |$($arg:ident : $typ:ty),+| $path:expr) => {
        pub async fn $name(&self, $($arg: $typ),+) -> Result<$resp, OpenRouterError> {
            self.$with($($arg,)+ RequestOptions::default()).await
        }

        pub async fn $with(
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
            .await
        }
    };
}

macro_rules! dyn_patch_auth {
    ($name:ident, $with:ident, $req:ty, $resp:ty, |$($arg:ident : $typ:ty),+| $path:expr) => {
        pub async fn $name(
            &self,
            $($arg: $typ,)+
            request: $req,
        ) -> Result<$resp, OpenRouterError> {
            self.$with($($arg,)+ request, RequestOptions::default()).await
        }

        pub async fn $with(
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
            .await
        }
    };
}

macro_rules! dyn_put_auth {
    ($name:ident, $with:ident, $req:ty, $resp:ty, |$($arg:ident : $typ:ty),+| $path:expr) => {
        pub async fn $name(
            &self,
            $($arg: $typ,)+
            request: $req,
        ) -> Result<$resp, OpenRouterError> {
            self.$with($($arg,)+ request, RequestOptions::default()).await
        }

        pub async fn $with(
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
            .await
        }
    };
}

macro_rules! dyn_post_auth {
    ($name:ident, $with:ident, $req:ty, $resp:ty, |$($arg:ident : $typ:ty),+| $path:expr) => {
        pub async fn $name(
            &self,
            $($arg: $typ,)+
            request: $req,
        ) -> Result<$resp, OpenRouterError> {
            self.$with($($arg,)+ request, RequestOptions::default()).await
        }

        pub async fn $with(
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
            .await
        }
    };
}

impl AsyncOpenRouterClient {
    dynamic_route_methods!(
        dyn_get_auth,
        dyn_get_public,
        dyn_delete_auth,
        dyn_patch_auth,
        dyn_put_auth,
        dyn_post_auth
    );

    pub async fn download_file_content(
        &self,
        file_id: &str,
    ) -> Result<BinaryResponse, OpenRouterError> {
        self.download_file_content_with_options(file_id, RequestOptions::default())
            .await
    }

    pub async fn download_file_content_with_options(
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
        .await
    }

    pub async fn download_video_content(
        &self,
        job_id: &str,
    ) -> Result<BinaryResponse, OpenRouterError> {
        self.download_video_content_with_options(job_id, RequestOptions::default())
            .await
    }

    pub async fn download_video_content_with_options(
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
        .await
    }
}

async fn parse_json_response<T: DeserializeOwned>(
    resp: reqwest::Response,
) -> Result<T, OpenRouterError> {
    let status = resp.status();
    if !status.is_success() {
        let headers = resp.headers().clone();
        let body = resp.text().await.unwrap_or_default();
        return Err(parse_api_error(status, &headers, body));
    }

    resp.json()
        .await
        .map_err(|e| OpenRouterError::Decode(e.to_string()))
}

async fn parse_binary_response(resp: reqwest::Response) -> Result<BinaryResponse, OpenRouterError> {
    let status = resp.status();
    let headers = resp.headers().clone();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
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
        .await
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
        let mut part = multipart::Part::stream_with_length(file.bytes, length);
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
    use super::AsyncOpenRouterClient;
    use crate::{
        AuthKeyExchangeRequest, ChatCompletionRequest, ChatMessage, DEFAULT_BASE_URL, HttpMethod,
        OpenRouterError, PaginationQuery, ProviderPreferences, RawJsonRequest, RequestOptions,
        TranscriptionFileRequest,
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

            if header_end.is_none()
                && let Some(end) = buf.windows(4).position(|window| window == b"\r\n\r\n")
            {
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

            if let Some(end) = header_end
                && buf.len() >= end + 4 + content_length
            {
                break;
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

    fn sample_chat_request() -> ChatCompletionRequest {
        ChatCompletionRequest::new(
            "openai/gpt-4o-mini",
            vec![
                ChatMessage::system("You are concise."),
                ChatMessage::user("Say hello."),
            ],
        )
        .temperature(0.2)
        .max_tokens(64)
        .provider(ProviderPreferences::new().require_parameters(true))
    }

    #[tokio::test]
    async fn default_base_url_is_current() {
        assert_eq!(DEFAULT_BASE_URL, "https://openrouter.ai/api/v1");
    }

    #[tokio::test]
    async fn chat_completion_posts_expected_json_and_parses_response() {
        let (base_url, request) = serve_once(
            "200 OK",
            r#"{"id":"gen-123","model":"openai/gpt-4o-mini","choices":[{"message":{"role":"assistant","content":"hello there"}}],"usage":{"prompt_tokens":12,"completion_tokens":3}}"#,
        )
        .await;
        let client = AsyncOpenRouterClient::try_new_unchecked_base_url_with_api_key(
            reqwest::Client::new(),
            base_url,
            "sk-test",
        )
        .unwrap();

        let response = client
            .create_chat_completion(sample_chat_request())
            .await
            .unwrap();

        assert_eq!(response.model.as_deref(), Some("openai/gpt-4o-mini"));
        assert_eq!(response.id.as_deref(), Some("gen-123"));
        assert_eq!(response.usage.unwrap().prompt_tokens, Some(12));

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
        assert_eq!(body["provider"]["require_parameters"], true);
    }

    #[tokio::test]
    async fn missing_api_key_errors_before_send() {
        let client = AsyncOpenRouterClient::try_new_unchecked_base_url(
            reqwest::Client::new(),
            "http://127.0.0.1:9",
        )
        .unwrap();

        let err = client
            .create_chat_completion(sample_chat_request())
            .await
            .unwrap_err();

        assert!(matches!(err, OpenRouterError::MissingApiKey));
    }

    #[tokio::test]
    async fn request_options_add_headers() {
        let (base_url, request) = serve_once("200 OK", r#"{"data":[]}"#).await;
        let client = AsyncOpenRouterClient::try_new_unchecked_base_url_with_api_key(
            reqwest::Client::new(),
            base_url,
            "sk-default",
        )
        .unwrap();

        let _ = client
            .list_keys_with_options(
                (),
                RequestOptions::new()
                    .with_http_referer("https://example.test")
                    .with_x_title("Example")
                    .with_session_id("sess-123")
                    .with_header("X-Custom", "value")
                    .with_api_key("sk-override"),
            )
            .await
            .unwrap();

        let recorded = request.await.unwrap();
        assert_eq!(
            recorded.header("http-referer"),
            Some("https://example.test")
        );
        assert_eq!(recorded.header("x-title"), Some("Example"));
        assert_eq!(recorded.header("x-session-id"), Some("sess-123"));
        assert_eq!(recorded.header("x-custom"), Some("value"));
        assert_eq!(recorded.header("authorization"), Some("Bearer sk-override"));
    }

    #[tokio::test]
    async fn auth_code_exchange_does_not_send_authorization() {
        let (base_url, request) = serve_once("200 OK", r#"{"key":"sk-new","user_id":null}"#).await;
        let client = AsyncOpenRouterClient::try_new_unchecked_base_url_with_api_key(
            reqwest::Client::new(),
            base_url,
            "sk-should-not-send",
        )
        .unwrap();

        let response = client
            .exchange_auth_code_for_api_key_with_options(
                AuthKeyExchangeRequest::new().with_field("code", "abc"),
                RequestOptions::new().without_auth(),
            )
            .await
            .unwrap();

        assert_eq!(response.extra["key"], "sk-new");
        let recorded = request.await.unwrap();
        assert_eq!(recorded.method, "POST");
        assert_eq!(recorded.path, "/auth/keys");
        assert_eq!(recorded.header("authorization"), None);
    }

    #[tokio::test]
    async fn api_error_includes_status_body_and_parsed_error() {
        let api_key = "sk-test-secret";
        let (base_url, request) = serve_once(
            "500 Internal Server Error",
            r#"{"error":{"message":"boom","type":"server_error"}}"#,
        )
        .await;
        let client = AsyncOpenRouterClient::try_new_unchecked_base_url_with_api_key(
            reqwest::Client::new(),
            base_url,
            api_key,
        )
        .unwrap();

        let err = client
            .create_chat_completion(sample_chat_request())
            .await
            .unwrap_err();

        match err {
            OpenRouterError::Api(api) => {
                assert_eq!(api.status, 500);
                assert!(api.body.contains("boom"));
                assert_eq!(api.error.unwrap().message.as_deref(), Some("boom"));
                assert!(!api.body.contains(api_key));
            }
            other => panic!("expected api error, got {other:?}"),
        }

        let recorded = request.await.unwrap();
        assert!(!recorded.body.contains(api_key));
    }

    #[tokio::test]
    async fn generation_cost_returns_none_for_not_yet_queryable_generation() {
        let (base_url, request) = serve_once("404 Not Found", "{}").await;
        let client = AsyncOpenRouterClient::try_new_unchecked_base_url_with_api_key(
            reqwest::Client::new(),
            base_url,
            "sk-cost",
        )
        .unwrap();

        let cost = client.generation_cost("gen-789").await.unwrap();

        assert_eq!(cost, None);
        let recorded = request.await.unwrap();
        assert_eq!(recorded.method, "GET");
        assert_eq!(recorded.path, "/generation?id=gen-789");
        assert_eq!(recorded.header("authorization"), Some("Bearer sk-cost"));
    }

    #[tokio::test]
    async fn list_workspace_members_sends_typed_pagination_query() {
        let (base_url, request) = serve_once("200 OK", r#"{"data":[],"total_count":0}"#).await;
        let client = AsyncOpenRouterClient::try_new_unchecked_base_url_with_api_key(
            reqwest::Client::new(),
            base_url,
            "sk-workspace",
        )
        .unwrap();

        let mut query = PaginationQuery::new();
        query.offset = Some(25);
        query.limit = Some(50);

        let response = client
            .list_workspace_members("production", query)
            .await
            .unwrap();

        assert_eq!(response.total_count, Some(0));
        let recorded = request.await.unwrap();
        assert_eq!(recorded.method, "GET");
        assert_eq!(
            recorded.path,
            "/workspaces/production/members?offset=25&limit=50"
        );
        assert_eq!(
            recorded.header("authorization"),
            Some("Bearer sk-workspace")
        );
    }

    #[tokio::test]
    async fn audio_transcription_file_posts_multipart_fields() {
        let (base_url, request) = serve_once("200 OK", r#"{"text":"hello"}"#).await;
        let client = AsyncOpenRouterClient::try_new_unchecked_base_url_with_api_key(
            reqwest::Client::new(),
            base_url,
            "sk-audio",
        )
        .unwrap();
        let mut transcript =
            TranscriptionFileRequest::new("openai/whisper-1", b"audio-bytes".as_slice())
                .with_file_name("clip.wav")
                .with_content_type("audio/wav");
        transcript.language = Some("en".to_owned());
        transcript.response_format = Some("json".to_owned());
        transcript.temperature = Some(0.1);

        let _ = client
            .create_audio_transcription_file(transcript)
            .await
            .unwrap();

        let recorded = request.await.unwrap();
        assert_eq!(recorded.method, "POST");
        assert_eq!(recorded.path, "/audio/transcriptions");
        assert_eq!(recorded.header("authorization"), Some("Bearer sk-audio"));
        assert!(recorded.body.contains("openai/whisper-1"));
        assert!(recorded.body.contains("clip.wav"));
        assert!(recorded.body.contains("audio-bytes"));
        assert!(recorded.body.contains("language"));
        assert!(recorded.body.contains("response_format"));
    }

    #[tokio::test]
    async fn raw_absolute_paths_are_rejected_before_send() {
        let client = AsyncOpenRouterClient::try_new_unchecked_base_url(
            reqwest::Client::new(),
            "http://127.0.0.1:9",
        )
        .unwrap();

        let err = client
            .raw_json(RawJsonRequest::new(
                HttpMethod::Get,
                "https://user:pass@example.test/secret",
            ))
            .await
            .unwrap_err();

        assert!(matches!(err, OpenRouterError::InvalidBaseUrl(_)));
    }

    #[tokio::test]
    #[ignore = "requires OPENROUTER_API_KEY and live OpenRouter access"]
    async fn live_smoke_get_current_key() {
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .expect("OPENROUTER_API_KEY must be set for live smoke tests");
        let client = AsyncOpenRouterClient::try_new_with_api_key(
            reqwest::Client::new(),
            DEFAULT_BASE_URL,
            api_key,
        )
        .unwrap();

        let _ = client
            .get_current_key(())
            .await
            .expect("live get_current_key request should succeed");
    }
}
