use reqwest::{Url, blocking::multipart};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{parse_api_error, reqwest_error_message};
use crate::routes::{HttpMethod, MultipartFile, RawJsonRequest, RawMultipartRequest};
use crate::streaming::BlockingSseStream;
use crate::transport::{
    QueryParams, endpoint_url_from_base, normalize_base_url, path_segment, with_query,
};
use crate::types::*;
use crate::{OpenRouterError, RequestOptions};

pub struct BlockingOpenRouterClient {
    http: reqwest::blocking::Client,
    base_url: Url,
}

impl BlockingOpenRouterClient {
    pub fn new(http: reqwest::blocking::Client, base_url: impl Into<String>) -> Self {
        Self::try_new(http, base_url).expect("invalid OpenRouter base URL")
    }

    pub fn try_new(
        http: reqwest::blocking::Client,
        base_url: impl Into<String>,
    ) -> Result<Self, OpenRouterError> {
        Ok(Self {
            http,
            base_url: normalize_base_url(base_url.into())
                .map_err(OpenRouterError::InvalidBaseUrl)?,
        })
    }

    pub fn http(&self) -> &reqwest::blocking::Client {
        &self.http
    }

    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    pub fn raw_json(
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
    }

    pub fn raw_binary(
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
    }

    pub fn raw_multipart(
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
    }

    fn request_builder(
        &self,
        method: HttpMethod,
        path: &str,
        api_key: Option<&str>,
        query: &[(String, String)],
        options: &RequestOptions,
    ) -> Result<reqwest::blocking::RequestBuilder, OpenRouterError> {
        let url = with_query(endpoint_url_from_base(&self.base_url, path)?, query);
        let mut builder = self.http.request(method.into(), url);
        if let Some(api_key) = api_key {
            builder = builder.bearer_auth(api_key);
        }
        options.apply_blocking(builder)
    }

    fn request_json_no_body<T: DeserializeOwned>(
        &self,
        method: HttpMethod,
        path: &str,
        api_key: Option<&str>,
        query: &[(String, String)],
        options: &RequestOptions,
    ) -> Result<T, OpenRouterError> {
        let resp = self
            .request_builder(method, path, api_key, query, options)?
            .send()
            .map_err(|e| OpenRouterError::Transport(reqwest_error_message(&e)))?;
        parse_json_response(resp)
    }

    fn request_json_body<B: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        method: HttpMethod,
        path: &str,
        api_key: Option<&str>,
        query: &[(String, String)],
        body: &B,
        options: &RequestOptions,
    ) -> Result<T, OpenRouterError> {
        let resp = self
            .request_builder(method, path, api_key, query, options)?
            .json(body)
            .send()
            .map_err(|e| OpenRouterError::Transport(reqwest_error_message(&e)))?;
        parse_json_response(resp)
    }

    fn request_json_value(
        &self,
        method: HttpMethod,
        path: &str,
        api_key: Option<&str>,
        query: &[(String, String)],
        body: Option<&Value>,
        options: &RequestOptions,
    ) -> Result<Value, OpenRouterError> {
        match body {
            Some(body) => self.request_json_body(method, path, api_key, query, body, options),
            None => self.request_json_no_body(method, path, api_key, query, options),
        }
    }

    fn request_binary(
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
        let resp = builder
            .send()
            .map_err(|e| OpenRouterError::Transport(reqwest_error_message(&e)))?;
        parse_binary_response(resp)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "transport helpers keep the HTTP request pieces explicit"
    )]
    fn request_multipart_value(
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
        let resp = self
            .request_builder(method, path, api_key, query, options)?
            .multipart(form)
            .send()
            .map_err(|e| OpenRouterError::Transport(reqwest_error_message(&e)))?;
        parse_json_response(resp)
    }

    fn stream_json_body<B: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        path: &str,
        api_key: &str,
        body: &B,
        options: &RequestOptions,
    ) -> Result<BlockingSseStream<T>, OpenRouterError> {
        let resp = self
            .request_builder(HttpMethod::Post, path, Some(api_key), &[], options)?
            .json(body)
            .send()
            .map_err(|e| OpenRouterError::Transport(reqwest_error_message(&e)))?;
        let status = resp.status();
        if !status.is_success() {
            let headers = resp.headers().clone();
            let body = resp.text().unwrap_or_default();
            return Err(parse_api_error(status, &headers, body));
        }
        Ok(BlockingSseStream::new(resp))
    }

    pub fn create_chat_completion(
        &self,
        api_key: &str,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, OpenRouterError> {
        self.create_chat_completion_with_options(api_key, request, RequestOptions::default())
    }

    pub fn create_chat_completion_with_options(
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
    }

    pub fn stream_chat_completion(
        &self,
        api_key: &str,
        mut request: ChatCompletionRequest,
    ) -> Result<BlockingSseStream<ChatStreamChunk>, OpenRouterError> {
        request.stream = Some(true);
        self.stream_chat_completion_with_options(api_key, request, RequestOptions::default())
    }

    pub fn stream_chat_completion_with_options(
        &self,
        api_key: &str,
        mut request: ChatCompletionRequest,
        options: RequestOptions,
    ) -> Result<BlockingSseStream<ChatStreamChunk>, OpenRouterError> {
        request.stream = Some(true);
        self.stream_json_body("chat/completions", api_key, &request, &options)
    }

    pub fn create_response(
        &self,
        api_key: &str,
        request: ResponsesRequest,
    ) -> Result<ResponsesResponse, OpenRouterError> {
        self.create_response_with_options(api_key, request, RequestOptions::default())
    }

    pub fn create_response_with_options(
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
    }

    pub fn stream_response(
        &self,
        api_key: &str,
        mut request: ResponsesRequest,
    ) -> Result<BlockingSseStream<StreamedResponsesEvent>, OpenRouterError> {
        request.stream = Some(true);
        self.stream_response_with_options(api_key, request, RequestOptions::default())
    }

    pub fn stream_response_with_options(
        &self,
        api_key: &str,
        mut request: ResponsesRequest,
        options: RequestOptions,
    ) -> Result<BlockingSseStream<StreamedResponsesEvent>, OpenRouterError> {
        request.stream = Some(true);
        self.stream_json_body("responses", api_key, &request, &options)
    }

    pub fn create_message(
        &self,
        api_key: &str,
        request: MessagesRequest,
    ) -> Result<MessagesResponse, OpenRouterError> {
        self.create_message_with_options(api_key, request, RequestOptions::default())
    }

    pub fn create_message_with_options(
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
    }

    pub fn stream_message(
        &self,
        api_key: &str,
        mut request: MessagesRequest,
    ) -> Result<BlockingSseStream<MessagesStreamEvent>, OpenRouterError> {
        request.stream = Some(true);
        self.stream_message_with_options(api_key, request, RequestOptions::default())
    }

    pub fn stream_message_with_options(
        &self,
        api_key: &str,
        mut request: MessagesRequest,
        options: RequestOptions,
    ) -> Result<BlockingSseStream<MessagesStreamEvent>, OpenRouterError> {
        request.stream = Some(true);
        self.stream_json_body("messages", api_key, &request, &options)
    }

    pub fn generation_cost(
        &self,
        api_key: &str,
        generation_id: &str,
    ) -> Result<Option<f64>, OpenRouterError> {
        match self.get_generation(api_key, generation_id) {
            Ok(generation) => Ok(generation.total_cost()),
            Err(err) if is_not_found(&err) => Ok(None),
            Err(err) => Err(err),
        }
    }
}

macro_rules! blocking_get_public {
    ($name:ident, $with:ident, $path:literal, $resp:ty) => {
        pub fn $name(&self, query: QueryParams) -> Result<$resp, OpenRouterError> {
            self.$with(query, RequestOptions::default())
        }

        pub fn $with(
            &self,
            query: QueryParams,
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            self.request_json_no_body(HttpMethod::Get, $path, None, &query, &options)
        }
    };
}

macro_rules! blocking_get_auth {
    ($name:ident, $with:ident, $path:literal, $resp:ty) => {
        pub fn $name(&self, api_key: &str, query: QueryParams) -> Result<$resp, OpenRouterError> {
            self.$with(api_key, query, RequestOptions::default())
        }

        pub fn $with(
            &self,
            api_key: &str,
            query: QueryParams,
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            self.request_json_no_body(HttpMethod::Get, $path, Some(api_key), &query, &options)
        }
    };
}

macro_rules! blocking_post_auth {
    ($name:ident, $with:ident, $path:literal, $req:ty, $resp:ty) => {
        pub fn $name(&self, api_key: &str, request: $req) -> Result<$resp, OpenRouterError> {
            self.$with(api_key, request, RequestOptions::default())
        }

        pub fn $with(
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
        }
    };
}

impl BlockingOpenRouterClient {
    blocking_get_auth!(
        get_user_activity,
        get_user_activity_with_options,
        "activity",
        ActivityResponse
    );
    blocking_get_auth!(
        get_analytics_meta,
        get_analytics_meta_with_options,
        "analytics/meta",
        AnalyticsMetaResponse
    );
    blocking_post_auth!(
        query_analytics,
        query_analytics_with_options,
        "analytics/query",
        AnalyticsQueryRequest,
        AnalyticsQueryResponse
    );
    blocking_post_auth!(
        create_audio_transcription,
        create_audio_transcription_with_options,
        "audio/transcriptions",
        TranscriptionRequest,
        TranscriptionResponse
    );
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
        self.request_json_body(HttpMethod::Post, "auth/keys", None, &[], &request, &options)
    }
    blocking_post_auth!(
        create_auth_key_code,
        create_auth_key_code_with_options,
        "auth/keys/code",
        AuthKeyCodeRequest,
        AuthKeyCodeResponse
    );
    blocking_get_public!(
        list_benchmarks,
        list_benchmarks_with_options,
        "benchmarks",
        BenchmarksResponse
    );
    blocking_get_auth!(
        list_byok_keys,
        list_byok_keys_with_options,
        "byok",
        ByokListResponse
    );
    blocking_post_auth!(
        create_byok_key,
        create_byok_key_with_options,
        "byok",
        ByokCreateRequest,
        ByokCreateResponse
    );
    blocking_get_public!(
        get_task_classifications,
        get_task_classifications_with_options,
        "classifications/task",
        TaskClassificationResponse
    );
    blocking_get_auth!(
        get_credits,
        get_credits_with_options,
        "credits",
        CreditsResponse
    );
    blocking_get_public!(
        get_app_rankings,
        get_app_rankings_with_options,
        "datasets/app-rankings",
        AppRankingsResponse
    );
    blocking_get_public!(
        get_rankings_daily,
        get_rankings_daily_with_options,
        "datasets/rankings-daily",
        RankingsDailyResponse
    );
    blocking_post_auth!(
        create_embeddings,
        create_embeddings_with_options,
        "embeddings",
        EmbeddingsRequest,
        EmbeddingsResponse
    );
    blocking_get_public!(
        list_embedding_models,
        list_embedding_models_with_options,
        "embeddings/models",
        EmbeddingModelsResponse
    );
    blocking_get_public!(
        list_zdr_endpoints,
        list_zdr_endpoints_with_options,
        "endpoints/zdr",
        EndpointsZdrResponse
    );
    blocking_get_auth!(
        list_files,
        list_files_with_options,
        "files",
        FileListResponse
    );
    blocking_get_auth!(
        list_guardrails,
        list_guardrails_with_options,
        "guardrails",
        GuardrailListResponse
    );
    blocking_post_auth!(
        create_guardrail,
        create_guardrail_with_options,
        "guardrails",
        GuardrailCreateRequest,
        GuardrailCreateResponse
    );
    blocking_get_auth!(
        list_key_assignments,
        list_key_assignments_with_options,
        "guardrails/assignments/keys",
        KeyAssignmentsResponse
    );
    blocking_get_auth!(
        list_member_assignments,
        list_member_assignments_with_options,
        "guardrails/assignments/members",
        MemberAssignmentsResponse
    );
    blocking_post_auth!(
        create_image,
        create_image_with_options,
        "images",
        ImageGenerationRequest,
        ImageGenerationResponse
    );
    blocking_get_public!(
        list_image_models,
        list_image_models_with_options,
        "images/models",
        ImageModelsResponse
    );
    blocking_get_auth!(
        get_current_key,
        get_current_key_with_options,
        "key",
        CurrentKeyResponse
    );
    blocking_get_auth!(list_keys, list_keys_with_options, "keys", KeyListResponse);
    blocking_post_auth!(
        create_key,
        create_key_with_options,
        "keys",
        KeyCreateRequest,
        KeyCreateResponse
    );
    blocking_get_public!(
        list_models,
        list_models_with_options,
        "models",
        ModelListResponse
    );
    blocking_get_public!(
        get_models_count,
        get_models_count_with_options,
        "models/count",
        ModelCountResponse
    );
    blocking_get_auth!(
        list_user_models,
        list_user_models_with_options,
        "models/user",
        UserModelsResponse
    );
    blocking_get_auth!(
        list_observability_destinations,
        list_observability_destinations_with_options,
        "observability/destinations",
        ObservabilityDestinationListResponse
    );
    blocking_post_auth!(
        create_observability_destination,
        create_observability_destination_with_options,
        "observability/destinations",
        ObservabilityDestinationCreateRequest,
        ObservabilityDestinationCreateResponse
    );
    blocking_get_auth!(
        list_organization_members,
        list_organization_members_with_options,
        "organization/members",
        OrganizationMembersResponse
    );
    blocking_get_auth!(
        list_presets,
        list_presets_with_options,
        "presets",
        PresetListResponse
    );
    blocking_get_public!(
        list_providers,
        list_providers_with_options,
        "providers",
        ProviderListResponse
    );
    blocking_post_auth!(
        create_rerank,
        create_rerank_with_options,
        "rerank",
        RerankRequest,
        RerankResponse
    );
    blocking_post_auth!(
        create_video,
        create_video_with_options,
        "videos",
        VideoGenerationRequest,
        VideoGenerationResponse
    );
    blocking_get_public!(
        list_video_models,
        list_video_models_with_options,
        "videos/models",
        VideoModelsResponse
    );
    blocking_get_auth!(
        list_workspaces,
        list_workspaces_with_options,
        "workspaces",
        WorkspaceListResponse
    );
    blocking_post_auth!(
        create_workspace,
        create_workspace_with_options,
        "workspaces",
        WorkspaceCreateRequest,
        WorkspaceCreateResponse
    );
}

impl BlockingOpenRouterClient {
    pub fn create_audio_speech(
        &self,
        api_key: &str,
        request: SpeechRequest,
    ) -> Result<BinaryResponse, OpenRouterError> {
        self.create_audio_speech_with_options(api_key, request, RequestOptions::default())
    }

    pub fn create_audio_speech_with_options(
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
    }

    pub fn upload_file(
        &self,
        api_key: &str,
        request: FileUploadRequest,
    ) -> Result<FileUploadResponse, OpenRouterError> {
        self.upload_file_with_options(api_key, request, RequestOptions::default())
    }

    pub fn upload_file_with_options(
        &self,
        api_key: &str,
        request: FileUploadRequest,
        options: RequestOptions,
    ) -> Result<FileUploadResponse, OpenRouterError> {
        let mut file = MultipartFile::new("file", request.bytes);
        file.file_name = request.file_name;
        file.content_type = request.content_type;
        let value = self.request_multipart_value(
            HttpMethod::Post,
            "files",
            Some(api_key),
            &[],
            vec![file],
            Vec::new(),
            &options,
        )?;
        serde_json::from_value(value).map_err(|e| OpenRouterError::Decode(e.to_string()))
    }

    pub fn get_generation(
        &self,
        api_key: &str,
        generation_id: &str,
    ) -> Result<GenerationResponse, OpenRouterError> {
        self.get_generation_with_options(api_key, generation_id, RequestOptions::default())
    }

    pub fn get_generation_with_options(
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
    }

    pub fn get_generation_content(
        &self,
        api_key: &str,
        generation_id: &str,
    ) -> Result<GenerationContentResponse, OpenRouterError> {
        self.get_generation_content_with_options(api_key, generation_id, RequestOptions::default())
    }

    pub fn get_generation_content_with_options(
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
    }
}

macro_rules! dyn_get_auth {
    ($name:ident, $with:ident, $resp:ty, |$($arg:ident : $typ:ty),+| $path:expr) => {
        pub fn $name(
            &self,
            api_key: &str,
            $($arg: $typ,)+
            query: QueryParams,
        ) -> Result<$resp, OpenRouterError> {
            self.$with(api_key, $($arg,)+ query, RequestOptions::default())
        }

        pub fn $with(
            &self,
            api_key: &str,
            $($arg: $typ,)+
            query: QueryParams,
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            let path = $path;
            self.request_json_no_body(HttpMethod::Get, &path, Some(api_key), &query, &options)
        }
    };
}

macro_rules! dyn_get_public {
    ($name:ident, $with:ident, $resp:ty, |$($arg:ident : $typ:ty),+| $path:expr) => {
        pub fn $name(
            &self,
            $($arg: $typ,)+
            query: QueryParams,
        ) -> Result<$resp, OpenRouterError> {
            self.$with($($arg,)+ query, RequestOptions::default())
        }

        pub fn $with(
            &self,
            $($arg: $typ,)+
            query: QueryParams,
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            let path = $path;
            self.request_json_no_body(HttpMethod::Get, &path, None, &query, &options)
        }
    };
}

macro_rules! dyn_delete_auth {
    ($name:ident, $with:ident, $resp:ty, |$($arg:ident : $typ:ty),+| $path:expr) => {
        pub fn $name(&self, api_key: &str, $($arg: $typ),+) -> Result<$resp, OpenRouterError> {
            self.$with(api_key, $($arg,)+ RequestOptions::default())
        }

        pub fn $with(
            &self,
            api_key: &str,
            $($arg: $typ,)+
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            let path = $path;
            self.request_json_no_body(HttpMethod::Delete, &path, Some(api_key), &[], &options)
        }
    };
}

macro_rules! dyn_patch_auth {
    ($name:ident, $with:ident, $req:ty, $resp:ty, |$($arg:ident : $typ:ty),+| $path:expr) => {
        pub fn $name(
            &self,
            api_key: &str,
            $($arg: $typ,)+
            request: $req,
        ) -> Result<$resp, OpenRouterError> {
            self.$with(api_key, $($arg,)+ request, RequestOptions::default())
        }

        pub fn $with(
            &self,
            api_key: &str,
            $($arg: $typ,)+
            request: $req,
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            let path = $path;
            self.request_json_body(HttpMethod::Patch, &path, Some(api_key), &[], &request, &options)
        }
    };
}

macro_rules! dyn_put_auth {
    ($name:ident, $with:ident, $req:ty, $resp:ty, |$($arg:ident : $typ:ty),+| $path:expr) => {
        pub fn $name(
            &self,
            api_key: &str,
            $($arg: $typ,)+
            request: $req,
        ) -> Result<$resp, OpenRouterError> {
            self.$with(api_key, $($arg,)+ request, RequestOptions::default())
        }

        pub fn $with(
            &self,
            api_key: &str,
            $($arg: $typ,)+
            request: $req,
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            let path = $path;
            self.request_json_body(HttpMethod::Put, &path, Some(api_key), &[], &request, &options)
        }
    };
}

macro_rules! dyn_post_auth {
    ($name:ident, $with:ident, $req:ty, $resp:ty, |$($arg:ident : $typ:ty),+| $path:expr) => {
        pub fn $name(
            &self,
            api_key: &str,
            $($arg: $typ,)+
            request: $req,
        ) -> Result<$resp, OpenRouterError> {
            self.$with(api_key, $($arg,)+ request, RequestOptions::default())
        }

        pub fn $with(
            &self,
            api_key: &str,
            $($arg: $typ,)+
            request: $req,
            options: RequestOptions,
        ) -> Result<$resp, OpenRouterError> {
            let path = $path;
            self.request_json_body(HttpMethod::Post, &path, Some(api_key), &[], &request, &options)
        }
    };
}

impl BlockingOpenRouterClient {
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

    pub fn download_file_content(
        &self,
        api_key: &str,
        file_id: &str,
    ) -> Result<BinaryResponse, OpenRouterError> {
        self.download_file_content_with_options(api_key, file_id, RequestOptions::default())
    }

    pub fn download_file_content_with_options(
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

    pub fn download_video_content(
        &self,
        api_key: &str,
        job_id: &str,
    ) -> Result<BinaryResponse, OpenRouterError> {
        self.download_video_content_with_options(api_key, job_id, RequestOptions::default())
    }

    pub fn download_video_content_with_options(
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
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc;
    use std::thread;

    use crate::streaming::SseMessage;
    use crate::{
        AuthKeyExchangeRequest, BlockingOpenRouterClient, ChatCompletionRequest, ChatMessage,
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
        let client =
            BlockingOpenRouterClient::try_new(reqwest::blocking::Client::new(), base_url).unwrap();

        let response = client
            .create_chat_completion(
                "sk-test",
                ChatCompletionRequest::new(
                    "openai/gpt-4o-mini",
                    vec![ChatMessage::user("Say hi.")],
                ),
            )
            .unwrap();

        assert_eq!(response.id.as_deref(), Some("gen-123"));
        let recorded = request.recv().unwrap();
        assert_eq!(recorded.method, "POST");
        assert_eq!(recorded.path, "/chat/completions");
        assert!(recorded.body.contains("openai/gpt-4o-mini"));
    }

    #[test]
    fn blocking_streaming_iterator_parses_events() {
        let (base_url, _request) = serve_once(
            "200 OK",
            "text/event-stream",
            "data: {\"id\":\"chunk-1\"}\n\ndata: [DONE]\n\n",
        );
        let client =
            BlockingOpenRouterClient::try_new(reqwest::blocking::Client::new(), base_url).unwrap();

        let mut stream = client
            .stream_chat_completion(
                "sk-test",
                ChatCompletionRequest::new(
                    "openai/gpt-4o-mini",
                    vec![ChatMessage::user("Say hi.")],
                ),
            )
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
        let client =
            BlockingOpenRouterClient::try_new(reqwest::blocking::Client::new(), base_url).unwrap();

        let response = client
            .exchange_auth_code_for_api_key(AuthKeyExchangeRequest::new().with_field("code", "abc"))
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
        let client =
            BlockingOpenRouterClient::try_new(reqwest::blocking::Client::new(), base_url).unwrap();

        let cost = client.generation_cost("sk-cost", "gen-789").unwrap();

        assert_eq!(cost, None);
        let recorded = request.recv().unwrap();
        assert_eq!(recorded.method, "GET");
        assert_eq!(recorded.path, "/generation?id=gen-789");
    }

    #[test]
    #[ignore = "requires OPENROUTER_API_KEY and live OpenRouter access"]
    fn live_smoke_get_current_key() {
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .expect("OPENROUTER_API_KEY must be set for live smoke tests");
        let client = BlockingOpenRouterClient::try_new(
            reqwest::blocking::Client::new(),
            crate::DEFAULT_BASE_URL,
        )
        .unwrap();

        let _ = client
            .get_current_key(&api_key, Vec::new())
            .expect("live get_current_key request should succeed");
    }
}
