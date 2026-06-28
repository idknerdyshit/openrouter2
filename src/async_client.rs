use reqwest::{Url, multipart};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{parse_api_error, reqwest_error_message};
use crate::observability::RequestTrace;
use crate::routes::{HttpMethod, MultipartFile, RawJsonRequest, RawMultipartRequest};
use crate::streaming::{AsyncSseStream, decode_async_sse};
use crate::transport::{
    QueryParams, endpoint_url_from_base, normalize_base_url, path_segment, with_query,
};
use crate::types::*;
use crate::{OpenRouterError, RequestOptions};

pub struct AsyncOpenRouterClient {
    http: reqwest::Client,
    base_url: Url,
}

impl AsyncOpenRouterClient {
    pub fn new(http: reqwest::Client, base_url: impl Into<String>) -> Self {
        Self::try_new(http, base_url).expect("invalid OpenRouter base URL")
    }

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

    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    pub async fn raw_json(
        &self,
        api_key: Option<&str>,
        request: RawJsonRequest,
    ) -> Result<Value, OpenRouterError> {
        self.request_json_value(
            request.method,
            &request.path,
            api_key,
            &request.query,
            request.body.as_ref(),
            &request.options,
        )
        .await
    }

    pub async fn raw_binary(
        &self,
        api_key: Option<&str>,
        request: RawJsonRequest,
    ) -> Result<BinaryResponse, OpenRouterError> {
        self.request_binary(
            request.method,
            &request.path,
            api_key,
            &request.query,
            request.body.as_ref(),
            &request.options,
        )
        .await
    }

    pub async fn raw_multipart(
        &self,
        api_key: Option<&str>,
        request: RawMultipartRequest,
    ) -> Result<Value, OpenRouterError> {
        self.request_multipart_value(
            request.method,
            &request.path,
            api_key,
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
        api_key: Option<&str>,
        query: &[(String, String)],
        options: &RequestOptions,
    ) -> Result<reqwest::RequestBuilder, OpenRouterError> {
        let url = with_query(endpoint_url_from_base(&self.base_url, path)?, query);
        let mut builder = self.http.request(method.into(), url);
        if let Some(api_key) = api_key {
            builder = builder.bearer_auth(api_key);
        }
        options.apply_async(builder)
    }

    async fn request_json_no_body<T: DeserializeOwned>(
        &self,
        method: HttpMethod,
        path: &str,
        api_key: Option<&str>,
        query: &[(String, String)],
        options: &RequestOptions,
    ) -> Result<T, OpenRouterError> {
        let trace = RequestTrace::start(method, path, query, api_key.is_some());
        let resp = match self
            .request_builder(method, path, api_key, query, options)?
            .send()
            .await
        {
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
        api_key: Option<&str>,
        query: &[(String, String)],
        body: &B,
        options: &RequestOptions,
    ) -> Result<T, OpenRouterError> {
        let trace = RequestTrace::start(method, path, query, api_key.is_some());
        let resp = match self
            .request_builder(method, path, api_key, query, options)?
            .json(body)
            .send()
            .await
        {
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
        api_key: Option<&str>,
        query: &[(String, String)],
        body: Option<&Value>,
        options: &RequestOptions,
    ) -> Result<Value, OpenRouterError> {
        match body {
            Some(body) => {
                self.request_json_body(method, path, api_key, query, body, options)
                    .await
            }
            None => {
                self.request_json_no_body(method, path, api_key, query, options)
                    .await
            }
        }
    }

    async fn request_binary(
        &self,
        method: HttpMethod,
        path: &str,
        api_key: Option<&str>,
        query: &[(String, String)],
        body: Option<&Value>,
        options: &RequestOptions,
    ) -> Result<BinaryResponse, OpenRouterError> {
        let mut builder = self.request_builder(method, path, api_key, query, options)?;
        if let Some(body) = body {
            builder = builder.json(body);
        }
        let trace = RequestTrace::start(method, path, query, api_key.is_some());
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
        api_key: Option<&str>,
        query: &[(String, String)],
        files: Vec<MultipartFile>,
        fields: Vec<(String, String)>,
        options: &RequestOptions,
    ) -> Result<Value, OpenRouterError> {
        let form = multipart_form(files, fields)?;
        let trace = RequestTrace::start(method, path, query, api_key.is_some());
        let resp = match self
            .request_builder(method, path, api_key, query, options)?
            .multipart(form)
            .send()
            .await
        {
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
        api_key: &str,
        body: &B,
        options: &RequestOptions,
    ) -> Result<AsyncSseStream<T>, OpenRouterError> {
        let trace = RequestTrace::start(HttpMethod::Post, path, &[], true);
        let resp = match self
            .request_builder(HttpMethod::Post, path, Some(api_key), &[], options)?
            .json(body)
            .send()
            .await
        {
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
        api_key: &str,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, OpenRouterError> {
        self.create_chat_completion_with_options(api_key, request, RequestOptions::default())
            .await
    }

    pub async fn create_chat_completion_with_options(
        &self,
        api_key: &str,
        request: ChatCompletionRequest,
        options: RequestOptions,
    ) -> Result<ChatCompletionResponse, OpenRouterError> {
        self.request_json_body(
            HttpMethod::Post,
            "chat/completions",
            Some(api_key),
            &[],
            &request,
            &options,
        )
        .await
    }

    pub async fn stream_chat_completion(
        &self,
        api_key: &str,
        mut request: ChatCompletionRequest,
    ) -> Result<AsyncSseStream<ChatStreamChunk>, OpenRouterError> {
        request.stream = Some(true);
        self.stream_chat_completion_with_options(api_key, request, RequestOptions::default())
            .await
    }

    pub async fn stream_chat_completion_with_options(
        &self,
        api_key: &str,
        mut request: ChatCompletionRequest,
        options: RequestOptions,
    ) -> Result<AsyncSseStream<ChatStreamChunk>, OpenRouterError> {
        request.stream = Some(true);
        self.stream_json_body("chat/completions", api_key, &request, &options)
            .await
    }

    pub async fn create_response(
        &self,
        api_key: &str,
        request: ResponsesRequest,
    ) -> Result<ResponsesResponse, OpenRouterError> {
        self.create_response_with_options(api_key, request, RequestOptions::default())
            .await
    }

    pub async fn create_response_with_options(
        &self,
        api_key: &str,
        request: ResponsesRequest,
        options: RequestOptions,
    ) -> Result<ResponsesResponse, OpenRouterError> {
        self.request_json_body(
            HttpMethod::Post,
            "responses",
            Some(api_key),
            &[],
            &request,
            &options,
        )
        .await
    }

    pub async fn stream_response(
        &self,
        api_key: &str,
        mut request: ResponsesRequest,
    ) -> Result<AsyncSseStream<StreamedResponsesEvent>, OpenRouterError> {
        request.stream = Some(true);
        self.stream_response_with_options(api_key, request, RequestOptions::default())
            .await
    }

    pub async fn stream_response_with_options(
        &self,
        api_key: &str,
        mut request: ResponsesRequest,
        options: RequestOptions,
    ) -> Result<AsyncSseStream<StreamedResponsesEvent>, OpenRouterError> {
        request.stream = Some(true);
        self.stream_json_body("responses", api_key, &request, &options)
            .await
    }

    pub async fn create_message(
        &self,
        api_key: &str,
        request: MessagesRequest,
    ) -> Result<MessagesResponse, OpenRouterError> {
        self.create_message_with_options(api_key, request, RequestOptions::default())
            .await
    }

    pub async fn create_message_with_options(
        &self,
        api_key: &str,
        request: MessagesRequest,
        options: RequestOptions,
    ) -> Result<MessagesResponse, OpenRouterError> {
        self.request_json_body(
            HttpMethod::Post,
            "messages",
            Some(api_key),
            &[],
            &request,
            &options,
        )
        .await
    }

    pub async fn stream_message(
        &self,
        api_key: &str,
        mut request: MessagesRequest,
    ) -> Result<AsyncSseStream<MessagesStreamEvent>, OpenRouterError> {
        request.stream = Some(true);
        self.stream_message_with_options(api_key, request, RequestOptions::default())
            .await
    }

    pub async fn stream_message_with_options(
        &self,
        api_key: &str,
        mut request: MessagesRequest,
        options: RequestOptions,
    ) -> Result<AsyncSseStream<MessagesStreamEvent>, OpenRouterError> {
        request.stream = Some(true);
        self.stream_json_body("messages", api_key, &request, &options)
            .await
    }

    pub async fn generation_cost(
        &self,
        api_key: &str,
        generation_id: &str,
    ) -> Result<Option<f64>, OpenRouterError> {
        match self.get_generation(api_key, generation_id).await {
            Ok(generation) => Ok(generation.total_cost()),
            Err(err) if is_not_found(&err) => Ok(None),
            Err(err) => Err(err),
        }
    }
}

macro_rules! async_get_public {
    ($name:ident, $with:ident, $path:literal, $resp:ty) => {
        pub async fn $name(&self, query: QueryParams) -> Result<$resp, OpenRouterError> {
            self.$with(query, RequestOptions::default()).await
        }

        pub async fn $with(
            &self,
            query: QueryParams,
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            self.request_json_no_body(HttpMethod::Get, $path, None, &query, &options)
                .await
        }
    };
}

macro_rules! async_get_auth {
    ($name:ident, $with:ident, $path:literal, $resp:ty) => {
        pub async fn $name(
            &self,
            api_key: &str,
            query: QueryParams,
        ) -> Result<$resp, OpenRouterError> {
            self.$with(api_key, query, RequestOptions::default()).await
        }

        pub async fn $with(
            &self,
            api_key: &str,
            query: QueryParams,
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            self.request_json_no_body(HttpMethod::Get, $path, Some(api_key), &query, &options)
                .await
        }
    };
}

macro_rules! async_post_auth {
    ($name:ident, $with:ident, $path:literal, $req:ty, $resp:ty) => {
        pub async fn $name(&self, api_key: &str, request: $req) -> Result<$resp, OpenRouterError> {
            self.$with(api_key, request, RequestOptions::default())
                .await
        }

        pub async fn $with(
            &self,
            api_key: &str,
            request: $req,
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            self.request_json_body(
                HttpMethod::Post,
                $path,
                Some(api_key),
                &[],
                &request,
                &options,
            )
            .await
        }
    };
}

impl AsyncOpenRouterClient {
    async_get_auth!(
        get_user_activity,
        get_user_activity_with_options,
        "activity",
        ActivityResponse
    );
    async_get_auth!(
        get_analytics_meta,
        get_analytics_meta_with_options,
        "analytics/meta",
        AnalyticsMetaResponse
    );
    async_post_auth!(
        query_analytics,
        query_analytics_with_options,
        "analytics/query",
        AnalyticsQueryRequest,
        AnalyticsQueryResponse
    );
    async_post_auth!(
        create_audio_transcription,
        create_audio_transcription_with_options,
        "audio/transcriptions",
        TranscriptionRequest,
        TranscriptionResponse
    );
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
        self.request_json_body(HttpMethod::Post, "auth/keys", None, &[], &request, &options)
            .await
    }
    async_post_auth!(
        create_auth_key_code,
        create_auth_key_code_with_options,
        "auth/keys/code",
        AuthKeyCodeRequest,
        AuthKeyCodeResponse
    );
    async_get_public!(
        list_benchmarks,
        list_benchmarks_with_options,
        "benchmarks",
        BenchmarksResponse
    );
    async_get_auth!(
        list_byok_keys,
        list_byok_keys_with_options,
        "byok",
        ByokListResponse
    );
    async_post_auth!(
        create_byok_key,
        create_byok_key_with_options,
        "byok",
        ByokCreateRequest,
        ByokCreateResponse
    );
    async_get_public!(
        get_task_classifications,
        get_task_classifications_with_options,
        "classifications/task",
        TaskClassificationResponse
    );
    async_get_auth!(
        get_credits,
        get_credits_with_options,
        "credits",
        CreditsResponse
    );
    async_get_public!(
        get_app_rankings,
        get_app_rankings_with_options,
        "datasets/app-rankings",
        AppRankingsResponse
    );
    async_get_public!(
        get_rankings_daily,
        get_rankings_daily_with_options,
        "datasets/rankings-daily",
        RankingsDailyResponse
    );
    async_post_auth!(
        create_embeddings,
        create_embeddings_with_options,
        "embeddings",
        EmbeddingsRequest,
        EmbeddingsResponse
    );
    async_get_public!(
        list_embedding_models,
        list_embedding_models_with_options,
        "embeddings/models",
        EmbeddingModelsResponse
    );
    async_get_public!(
        list_zdr_endpoints,
        list_zdr_endpoints_with_options,
        "endpoints/zdr",
        EndpointsZdrResponse
    );
    async_get_auth!(
        list_files,
        list_files_with_options,
        "files",
        FileListResponse
    );
    async_get_auth!(
        list_guardrails,
        list_guardrails_with_options,
        "guardrails",
        GuardrailListResponse
    );
    async_post_auth!(
        create_guardrail,
        create_guardrail_with_options,
        "guardrails",
        GuardrailCreateRequest,
        GuardrailCreateResponse
    );
    async_get_auth!(
        list_key_assignments,
        list_key_assignments_with_options,
        "guardrails/assignments/keys",
        KeyAssignmentsResponse
    );
    async_get_auth!(
        list_member_assignments,
        list_member_assignments_with_options,
        "guardrails/assignments/members",
        MemberAssignmentsResponse
    );
    async_post_auth!(
        create_image,
        create_image_with_options,
        "images",
        ImageGenerationRequest,
        ImageGenerationResponse
    );
    async_get_public!(
        list_image_models,
        list_image_models_with_options,
        "images/models",
        ImageModelsResponse
    );
    async_get_auth!(
        get_current_key,
        get_current_key_with_options,
        "key",
        CurrentKeyResponse
    );
    async_get_auth!(list_keys, list_keys_with_options, "keys", KeyListResponse);
    async_post_auth!(
        create_key,
        create_key_with_options,
        "keys",
        KeyCreateRequest,
        KeyCreateResponse
    );
    async_get_public!(
        list_models,
        list_models_with_options,
        "models",
        ModelListResponse
    );
    async_get_public!(
        get_models_count,
        get_models_count_with_options,
        "models/count",
        ModelCountResponse
    );
    async_get_auth!(
        list_user_models,
        list_user_models_with_options,
        "models/user",
        UserModelsResponse
    );
    async_get_auth!(
        list_observability_destinations,
        list_observability_destinations_with_options,
        "observability/destinations",
        ObservabilityDestinationListResponse
    );
    async_post_auth!(
        create_observability_destination,
        create_observability_destination_with_options,
        "observability/destinations",
        ObservabilityDestinationCreateRequest,
        ObservabilityDestinationCreateResponse
    );
    async_get_auth!(
        list_organization_members,
        list_organization_members_with_options,
        "organization/members",
        OrganizationMembersResponse
    );
    async_get_auth!(
        list_presets,
        list_presets_with_options,
        "presets",
        PresetListResponse
    );
    async_get_public!(
        list_providers,
        list_providers_with_options,
        "providers",
        ProviderListResponse
    );
    async_post_auth!(
        create_rerank,
        create_rerank_with_options,
        "rerank",
        RerankRequest,
        RerankResponse
    );
    async_post_auth!(
        create_video,
        create_video_with_options,
        "videos",
        VideoGenerationRequest,
        VideoGenerationResponse
    );
    async_get_public!(
        list_video_models,
        list_video_models_with_options,
        "videos/models",
        VideoModelsResponse
    );
    async_get_auth!(
        list_workspaces,
        list_workspaces_with_options,
        "workspaces",
        WorkspaceListResponse
    );
    async_post_auth!(
        create_workspace,
        create_workspace_with_options,
        "workspaces",
        WorkspaceCreateRequest,
        WorkspaceCreateResponse
    );
}

impl AsyncOpenRouterClient {
    pub async fn create_audio_speech(
        &self,
        api_key: &str,
        request: SpeechRequest,
    ) -> Result<BinaryResponse, OpenRouterError> {
        self.create_audio_speech_with_options(api_key, request, RequestOptions::default())
            .await
    }

    pub async fn create_audio_speech_with_options(
        &self,
        api_key: &str,
        request: SpeechRequest,
        options: RequestOptions,
    ) -> Result<BinaryResponse, OpenRouterError> {
        let body =
            serde_json::to_value(request).map_err(|e| OpenRouterError::Decode(e.to_string()))?;
        self.request_binary(
            HttpMethod::Post,
            "audio/speech",
            Some(api_key),
            &[],
            Some(&body),
            &options,
        )
        .await
    }

    pub async fn upload_file(
        &self,
        api_key: &str,
        request: FileUploadRequest,
    ) -> Result<FileUploadResponse, OpenRouterError> {
        self.upload_file_with_options(api_key, request, RequestOptions::default())
            .await
    }

    pub async fn upload_file_with_options(
        &self,
        api_key: &str,
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
                Some(api_key),
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
        api_key: &str,
        generation_id: &str,
    ) -> Result<GenerationResponse, OpenRouterError> {
        self.get_generation_with_options(api_key, generation_id, RequestOptions::default())
            .await
    }

    pub async fn get_generation_with_options(
        &self,
        api_key: &str,
        generation_id: &str,
        options: RequestOptions,
    ) -> Result<GenerationResponse, OpenRouterError> {
        self.request_json_no_body(
            HttpMethod::Get,
            "generation",
            Some(api_key),
            &[("id".to_owned(), generation_id.to_owned())],
            &options,
        )
        .await
    }

    pub async fn get_generation_content(
        &self,
        api_key: &str,
        generation_id: &str,
    ) -> Result<GenerationContentResponse, OpenRouterError> {
        self.get_generation_content_with_options(api_key, generation_id, RequestOptions::default())
            .await
    }

    pub async fn get_generation_content_with_options(
        &self,
        api_key: &str,
        generation_id: &str,
        options: RequestOptions,
    ) -> Result<GenerationContentResponse, OpenRouterError> {
        self.request_json_no_body(
            HttpMethod::Get,
            "generation/content",
            Some(api_key),
            &[("id".to_owned(), generation_id.to_owned())],
            &options,
        )
        .await
    }
}

macro_rules! dyn_get_auth {
    ($name:ident, $with:ident, $resp:ty, |$($arg:ident : $typ:ty),+| $path:expr) => {
        pub async fn $name(
            &self,
            api_key: &str,
            $($arg: $typ,)+
            query: QueryParams,
        ) -> Result<$resp, OpenRouterError> {
            self.$with(api_key, $($arg,)+ query, RequestOptions::default()).await
        }

        pub async fn $with(
            &self,
            api_key: &str,
            $($arg: $typ,)+
            query: QueryParams,
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            let path = $path;
            self.request_json_no_body(HttpMethod::Get, &path, Some(api_key), &query, &options).await
        }
    };
}

macro_rules! dyn_get_public {
    ($name:ident, $with:ident, $resp:ty, |$($arg:ident : $typ:ty),+| $path:expr) => {
        pub async fn $name(
            &self,
            $($arg: $typ,)+
            query: QueryParams,
        ) -> Result<$resp, OpenRouterError> {
            self.$with($($arg,)+ query, RequestOptions::default()).await
        }

        pub async fn $with(
            &self,
            $($arg: $typ,)+
            query: QueryParams,
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            let path = $path;
            self.request_json_no_body(HttpMethod::Get, &path, None, &query, &options).await
        }
    };
}

macro_rules! dyn_delete_auth {
    ($name:ident, $with:ident, $resp:ty, |$($arg:ident : $typ:ty),+| $path:expr) => {
        pub async fn $name(&self, api_key: &str, $($arg: $typ),+) -> Result<$resp, OpenRouterError> {
            self.$with(api_key, $($arg,)+ RequestOptions::default()).await
        }

        pub async fn $with(
            &self,
            api_key: &str,
            $($arg: $typ,)+
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            let path = $path;
            self.request_json_no_body(HttpMethod::Delete, &path, Some(api_key), &[], &options).await
        }
    };
}

macro_rules! dyn_patch_auth {
    ($name:ident, $with:ident, $req:ty, $resp:ty, |$($arg:ident : $typ:ty),+| $path:expr) => {
        pub async fn $name(
            &self,
            api_key: &str,
            $($arg: $typ,)+
            request: $req,
        ) -> Result<$resp, OpenRouterError> {
            self.$with(api_key, $($arg,)+ request, RequestOptions::default()).await
        }

        pub async fn $with(
            &self,
            api_key: &str,
            $($arg: $typ,)+
            request: $req,
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            let path = $path;
            self.request_json_body(HttpMethod::Patch, &path, Some(api_key), &[], &request, &options).await
        }
    };
}

macro_rules! dyn_put_auth {
    ($name:ident, $with:ident, $req:ty, $resp:ty, |$($arg:ident : $typ:ty),+| $path:expr) => {
        pub async fn $name(
            &self,
            api_key: &str,
            $($arg: $typ,)+
            request: $req,
        ) -> Result<$resp, OpenRouterError> {
            self.$with(api_key, $($arg,)+ request, RequestOptions::default()).await
        }

        pub async fn $with(
            &self,
            api_key: &str,
            $($arg: $typ,)+
            request: $req,
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            let path = $path;
            self.request_json_body(HttpMethod::Put, &path, Some(api_key), &[], &request, &options).await
        }
    };
}

macro_rules! dyn_post_auth {
    ($name:ident, $with:ident, $req:ty, $resp:ty, |$($arg:ident : $typ:ty),+| $path:expr) => {
        pub async fn $name(
            &self,
            api_key: &str,
            $($arg: $typ,)+
            request: $req,
        ) -> Result<$resp, OpenRouterError> {
            self.$with(api_key, $($arg,)+ request, RequestOptions::default()).await
        }

        pub async fn $with(
            &self,
            api_key: &str,
            $($arg: $typ,)+
            request: $req,
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            let path = $path;
            self.request_json_body(HttpMethod::Post, &path, Some(api_key), &[], &request, &options).await
        }
    };
}

impl AsyncOpenRouterClient {
    dyn_get_auth!(
        get_byok_key,
        get_byok_key_with_options,
        ByokResponse,
        |id: &str| format!("byok/{}", path_segment(id))
    );
    dyn_delete_auth!(
        delete_byok_key,
        delete_byok_key_with_options,
        ByokDeleteResponse,
        |id: &str| format!("byok/{}", path_segment(id))
    );
    dyn_patch_auth!(
        update_byok_key,
        update_byok_key_with_options,
        ByokUpdateRequest,
        ByokUpdateResponse,
        |id: &str| format!("byok/{}", path_segment(id))
    );

    dyn_get_auth!(
        get_file_metadata,
        get_file_metadata_with_options,
        FileMetadataResponse,
        |file_id: &str| format!("files/{}", path_segment(file_id))
    );
    dyn_delete_auth!(
        delete_file,
        delete_file_with_options,
        FileDeleteResponse,
        |file_id: &str| format!("files/{}", path_segment(file_id))
    );

    pub async fn download_file_content(
        &self,
        api_key: &str,
        file_id: &str,
    ) -> Result<BinaryResponse, OpenRouterError> {
        self.download_file_content_with_options(api_key, file_id, RequestOptions::default())
            .await
    }

    pub async fn download_file_content_with_options(
        &self,
        api_key: &str,
        file_id: &str,
        options: RequestOptions,
    ) -> Result<BinaryResponse, OpenRouterError> {
        self.request_binary(
            HttpMethod::Get,
            &format!("files/{}/content", path_segment(file_id)),
            Some(api_key),
            &[],
            None,
            &options,
        )
        .await
    }

    dyn_get_auth!(
        get_guardrail,
        get_guardrail_with_options,
        GuardrailResponse,
        |id: &str| format!("guardrails/{}", path_segment(id))
    );
    dyn_delete_auth!(
        delete_guardrail,
        delete_guardrail_with_options,
        GuardrailDeleteResponse,
        |id: &str| format!("guardrails/{}", path_segment(id))
    );
    dyn_patch_auth!(
        update_guardrail,
        update_guardrail_with_options,
        GuardrailUpdateRequest,
        GuardrailUpdateResponse,
        |id: &str| format!("guardrails/{}", path_segment(id))
    );
    dyn_get_auth!(
        list_guardrail_key_assignments,
        list_guardrail_key_assignments_with_options,
        KeyAssignmentsResponse,
        |id: &str| format!("guardrails/{}/assignments/keys", path_segment(id))
    );
    dyn_post_auth!(
        bulk_assign_keys_to_guardrail,
        bulk_assign_keys_to_guardrail_with_options,
        BulkAssignKeysRequest,
        BulkAssignKeysResponse,
        |id: &str| format!("guardrails/{}/assignments/keys", path_segment(id))
    );
    dyn_post_auth!(
        bulk_unassign_keys_from_guardrail,
        bulk_unassign_keys_from_guardrail_with_options,
        BulkUnassignKeysRequest,
        BulkUnassignKeysResponse,
        |id: &str| format!("guardrails/{}/assignments/keys/remove", path_segment(id))
    );
    dyn_get_auth!(
        list_guardrail_member_assignments,
        list_guardrail_member_assignments_with_options,
        MemberAssignmentsResponse,
        |id: &str| format!("guardrails/{}/assignments/members", path_segment(id))
    );
    dyn_post_auth!(
        bulk_assign_members_to_guardrail,
        bulk_assign_members_to_guardrail_with_options,
        BulkAssignMembersRequest,
        BulkAssignMembersResponse,
        |id: &str| format!("guardrails/{}/assignments/members", path_segment(id))
    );
    dyn_post_auth!(
        bulk_unassign_members_from_guardrail,
        bulk_unassign_members_from_guardrail_with_options,
        BulkUnassignMembersRequest,
        BulkUnassignMembersResponse,
        |id: &str| format!("guardrails/{}/assignments/members/remove", path_segment(id))
    );

    dyn_get_public!(
        list_image_model_endpoints,
        list_image_model_endpoints_with_options,
        ImageModelEndpointsResponse,
        |author: &str, slug: &str| format!(
            "images/models/{}/{}/endpoints",
            path_segment(author),
            path_segment(slug)
        )
    );
    dyn_get_auth!(
        get_key,
        get_key_with_options,
        KeyResponse,
        |hash: &str| format!("keys/{}", path_segment(hash))
    );
    dyn_delete_auth!(
        delete_key,
        delete_key_with_options,
        KeyDeleteResponse,
        |hash: &str| format!("keys/{}", path_segment(hash))
    );
    dyn_patch_auth!(
        update_key,
        update_key_with_options,
        KeyUpdateRequest,
        KeyUpdateResponse,
        |hash: &str| format!("keys/{}", path_segment(hash))
    );

    dyn_get_public!(
        get_model,
        get_model_with_options,
        ModelResponse,
        |author: &str, slug: &str| format!("model/{}/{}", path_segment(author), path_segment(slug))
    );
    dyn_get_public!(
        list_model_endpoints,
        list_model_endpoints_with_options,
        ModelEndpointsResponse,
        |author: &str, slug: &str| format!(
            "models/{}/{}/endpoints",
            path_segment(author),
            path_segment(slug)
        )
    );

    dyn_get_auth!(
        get_observability_destination,
        get_observability_destination_with_options,
        ObservabilityDestinationResponse,
        |id: &str| format!("observability/destinations/{}", path_segment(id))
    );
    dyn_delete_auth!(
        delete_observability_destination,
        delete_observability_destination_with_options,
        ObservabilityDestinationDeleteResponse,
        |id: &str| format!("observability/destinations/{}", path_segment(id))
    );
    dyn_patch_auth!(
        update_observability_destination,
        update_observability_destination_with_options,
        ObservabilityDestinationUpdateRequest,
        ObservabilityDestinationUpdateResponse,
        |id: &str| format!("observability/destinations/{}", path_segment(id))
    );

    dyn_get_auth!(
        get_preset,
        get_preset_with_options,
        PresetResponse,
        |slug: &str| format!("presets/{}", path_segment(slug))
    );
    dyn_post_auth!(
        create_preset_from_chat_completion,
        create_preset_from_chat_completion_with_options,
        ChatCompletionRequest,
        PresetCreateFromInferenceResponse,
        |slug: &str| format!("presets/{}/chat/completions", path_segment(slug))
    );
    dyn_post_auth!(
        create_preset_from_message,
        create_preset_from_message_with_options,
        MessagesRequest,
        PresetCreateFromInferenceResponse,
        |slug: &str| format!("presets/{}/messages", path_segment(slug))
    );
    dyn_post_auth!(
        create_preset_from_response,
        create_preset_from_response_with_options,
        ResponsesRequest,
        PresetCreateFromInferenceResponse,
        |slug: &str| format!("presets/{}/responses", path_segment(slug))
    );
    dyn_get_auth!(
        list_preset_versions,
        list_preset_versions_with_options,
        PresetVersionListResponse,
        |slug: &str| format!("presets/{}/versions", path_segment(slug))
    );
    dyn_get_auth!(
        get_preset_version,
        get_preset_version_with_options,
        PresetVersionResponse,
        |slug: &str, version: &str| format!(
            "presets/{}/versions/{}",
            path_segment(slug),
            path_segment(version)
        )
    );

    dyn_get_auth!(
        get_video,
        get_video_with_options,
        VideoStatusResponse,
        |job_id: &str| format!("videos/{}", path_segment(job_id))
    );

    pub async fn download_video_content(
        &self,
        api_key: &str,
        job_id: &str,
    ) -> Result<BinaryResponse, OpenRouterError> {
        self.download_video_content_with_options(api_key, job_id, RequestOptions::default())
            .await
    }

    pub async fn download_video_content_with_options(
        &self,
        api_key: &str,
        job_id: &str,
        options: RequestOptions,
    ) -> Result<BinaryResponse, OpenRouterError> {
        self.request_binary(
            HttpMethod::Get,
            &format!("videos/{}/content", path_segment(job_id)),
            Some(api_key),
            &[],
            None,
            &options,
        )
        .await
    }

    dyn_get_auth!(
        get_workspace,
        get_workspace_with_options,
        WorkspaceResponse,
        |id: &str| format!("workspaces/{}", path_segment(id))
    );
    dyn_delete_auth!(
        delete_workspace,
        delete_workspace_with_options,
        WorkspaceDeleteResponse,
        |id: &str| format!("workspaces/{}", path_segment(id))
    );
    dyn_patch_auth!(
        update_workspace,
        update_workspace_with_options,
        WorkspaceUpdateRequest,
        WorkspaceUpdateResponse,
        |id: &str| format!("workspaces/{}", path_segment(id))
    );
    dyn_get_auth!(
        list_workspace_budgets,
        list_workspace_budgets_with_options,
        WorkspaceBudgetListResponse,
        |id: &str| format!("workspaces/{}/budgets", path_segment(id))
    );
    dyn_delete_auth!(
        delete_workspace_budget,
        delete_workspace_budget_with_options,
        WorkspaceBudgetDeleteResponse,
        |id: &str, interval: &str| format!(
            "workspaces/{}/budgets/{}",
            path_segment(id),
            path_segment(interval)
        )
    );
    dyn_put_auth!(
        upsert_workspace_budget,
        upsert_workspace_budget_with_options,
        WorkspaceBudgetUpsertRequest,
        WorkspaceBudgetUpsertResponse,
        |id: &str, interval: &str| format!(
            "workspaces/{}/budgets/{}",
            path_segment(id),
            path_segment(interval)
        )
    );
    dyn_post_auth!(
        bulk_add_workspace_members,
        bulk_add_workspace_members_with_options,
        BulkAddWorkspaceMembersRequest,
        BulkAddWorkspaceMembersResponse,
        |id: &str| format!("workspaces/{}/members/add", path_segment(id))
    );
    dyn_post_auth!(
        bulk_remove_workspace_members,
        bulk_remove_workspace_members_with_options,
        BulkRemoveWorkspaceMembersRequest,
        BulkRemoveWorkspaceMembersResponse,
        |id: &str| format!("workspaces/{}/members/remove", path_segment(id))
    );
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
        let mut part = multipart::Part::bytes(file.bytes.to_vec());
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
        AuthKeyExchangeRequest, ChatCompletionRequest, ChatMessage, DEFAULT_BASE_URL,
        OpenRouterError, ProviderPreferences, RequestOptions,
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
        let client = AsyncOpenRouterClient::try_new(reqwest::Client::new(), base_url).unwrap();

        let response = client
            .create_chat_completion("sk-test", sample_chat_request())
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
    async fn request_options_add_headers() {
        let (base_url, request) = serve_once("200 OK", r#"{"data":[]}"#).await;
        let client = AsyncOpenRouterClient::try_new(reqwest::Client::new(), base_url).unwrap();

        let _ = client
            .list_keys_with_options(
                "sk-test",
                Vec::new(),
                RequestOptions::new()
                    .with_http_referer("https://example.test")
                    .with_x_title("Example"),
            )
            .await
            .unwrap();

        let recorded = request.await.unwrap();
        assert_eq!(
            recorded.header("http-referer"),
            Some("https://example.test")
        );
        assert_eq!(recorded.header("x-title"), Some("Example"));
    }

    #[tokio::test]
    async fn auth_code_exchange_does_not_send_authorization() {
        let (base_url, request) = serve_once("200 OK", r#"{"key":"sk-new","user_id":null}"#).await;
        let client = AsyncOpenRouterClient::try_new(reqwest::Client::new(), base_url).unwrap();

        let response = client
            .exchange_auth_code_for_api_key(AuthKeyExchangeRequest::new().with_field("code", "abc"))
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
        let client = AsyncOpenRouterClient::try_new(reqwest::Client::new(), base_url).unwrap();

        let err = client
            .create_chat_completion(api_key, sample_chat_request())
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
        let client = AsyncOpenRouterClient::try_new(reqwest::Client::new(), base_url).unwrap();

        let cost = client.generation_cost("sk-cost", "gen-789").await.unwrap();

        assert_eq!(cost, None);
        let recorded = request.await.unwrap();
        assert_eq!(recorded.method, "GET");
        assert_eq!(recorded.path, "/generation?id=gen-789");
        assert_eq!(recorded.header("authorization"), Some("Bearer sk-cost"));
    }

    #[tokio::test]
    #[ignore = "requires OPENROUTER_API_KEY and live OpenRouter access"]
    async fn live_smoke_get_current_key() {
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .expect("OPENROUTER_API_KEY must be set for live smoke tests");
        let client =
            AsyncOpenRouterClient::try_new(reqwest::Client::new(), DEFAULT_BASE_URL).unwrap();

        let _ = client
            .get_current_key(&api_key, Vec::new())
            .await
            .expect("live get_current_key request should succeed");
    }
}
