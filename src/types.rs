use std::collections::BTreeMap;

use bytes::Bytes;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

pub type JsonObject = BTreeMap<String, Value>;
pub type QueryParams = crate::transport::QueryParams;

macro_rules! string_enum {
    ($name:ident { $($variant:ident => $wire:literal,)* }) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub enum $name {
            $($variant,)*
            Unknown(String),
        }

        impl $name {
            pub fn as_str(&self) -> &str {
                match self {
                    $(Self::$variant => $wire,)*
                    Self::Unknown(value) => value,
                }
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                match value {
                    $($wire => Self::$variant,)*
                    other => Self::Unknown(other.to_owned()),
                }
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::from(value.as_str())
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Ok(Self::from(value))
            }
        }
    };
}

macro_rules! json_object_type {
    ($name:ident) => {
        #[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
        pub struct $name {
            #[serde(flatten)]
            pub extra: JsonObject,
        }

        impl $name {
            pub fn new() -> Self {
                Self::default()
            }

            pub fn with_field(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
                self.extra.insert(key.into(), value.into());
                self
            }
        }
    };
}

macro_rules! query_struct {
    ($name:ident { $($field:ident : $typ:ty => $wire:literal,)* }) => {
        #[derive(Debug, Clone, Default, PartialEq)]
        pub struct $name {
            $(pub $field: Option<$typ>,)*
            pub extra: QueryParams,
        }

        impl $name {
            pub fn new() -> Self {
                Self::default()
            }

            pub fn with_extra_query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
                self.extra.push((key.into(), value.into()));
                self
            }
        }

        impl crate::transport::IntoQueryParams for $name {
            fn into_query_params(self) -> QueryParams {
                let mut query = Vec::new();
                $(
                    if let Some(value) = self.$field {
                        query.push(($wire.to_owned(), value.to_string()));
                    }
                )*
                query.extend(self.extra);
                query
            }
        }
    };
}

query_struct!(PaginationQuery {
    offset: u32 => "offset",
    limit: u32 => "limit",
});

query_struct!(ActivityQuery {
    date: String => "date",
    api_key_hash: String => "api_key_hash",
    user_id: String => "user_id",
});

query_struct!(BenchmarksQuery {
    source: String => "source",
    task_type: String => "task_type",
    arena: String => "arena",
    category: String => "category",
    max_results: u32 => "max_results",
});

query_struct!(TaskClassificationsQuery {
    window: String => "window",
});

query_struct!(AppRankingsQuery {
    category: String => "category",
    subcategory: String => "subcategory",
    sort: String => "sort",
    start_date: String => "start_date",
    end_date: String => "end_date",
    limit: u32 => "limit",
    offset: u32 => "offset",
});

query_struct!(RankingsDailyQuery {
    start_date: String => "start_date",
    end_date: String => "end_date",
});

query_struct!(FilesQuery {
    limit: u32 => "limit",
    cursor: String => "cursor",
    workspace_id: String => "workspace_id",
});

query_struct!(WorkspaceScopedPaginationQuery {
    offset: u32 => "offset",
    limit: u32 => "limit",
    workspace_id: String => "workspace_id",
});

query_struct!(KeysQuery {
    include_disabled: bool => "include_disabled",
    offset: u32 => "offset",
    workspace_id: String => "workspace_id",
});

query_struct!(ModelsQuery {
    category: String => "category",
    supported_parameters: String => "supported_parameters",
    output_modalities: String => "output_modalities",
    sort: String => "sort",
    q: String => "q",
    input_modalities: String => "input_modalities",
    context: u32 => "context",
    min_price: f64 => "min_price",
    max_price: f64 => "max_price",
    arch: String => "arch",
    model_authors: String => "model_authors",
    providers: String => "providers",
    distillable: String => "distillable",
    zdr: String => "zdr",
    region: String => "region",
    min_output_price: f64 => "min_output_price",
    max_output_price: f64 => "max_output_price",
    min_age_days: u32 => "min_age_days",
    max_age_days: u32 => "max_age_days",
    min_intelligence_index: f64 => "min_intelligence_index",
    max_intelligence_index: f64 => "max_intelligence_index",
    min_coding_index: f64 => "min_coding_index",
    max_coding_index: f64 => "max_coding_index",
    min_agentic_index: f64 => "min_agentic_index",
    max_agentic_index: f64 => "max_agentic_index",
    min_tool_success_rate: f64 => "min_tool_success_rate",
    max_tool_success_rate: f64 => "max_tool_success_rate",
});

query_struct!(ModelCountQuery {
    output_modalities: String => "output_modalities",
});

query_struct!(VideoContentQuery {
    index: u32 => "index",
});

pub type ByokKeysQuery = WorkspaceScopedPaginationQuery;
pub type GuardrailsQuery = WorkspaceScopedPaginationQuery;
pub type ObservabilityDestinationsQuery = WorkspaceScopedPaginationQuery;
pub type KeyAssignmentsQuery = PaginationQuery;
pub type MemberAssignmentsQuery = PaginationQuery;
pub type OrganizationMembersQuery = PaginationQuery;
pub type PresetsQuery = PaginationQuery;
pub type PresetVersionsQuery = PaginationQuery;
pub type WorkspacesQuery = PaginationQuery;
pub type WorkspaceMembersQuery = PaginationQuery;

string_enum!(ChatRole {
    System => "system",
    User => "user",
    Assistant => "assistant",
    Developer => "developer",
    Tool => "tool",
});

string_enum!(FinishReason {
    Stop => "stop",
    Length => "length",
    ToolCalls => "tool_calls",
    ContentFilter => "content_filter",
    Error => "error",
});

string_enum!(ServiceTier {
    Auto => "auto",
    Default => "default",
    Flex => "flex",
    Priority => "priority",
    Scale => "scale",
});

string_enum!(DataCollection {
    Allow => "allow",
    Deny => "deny",
});

string_enum!(ProviderSort {
    Price => "price",
    Throughput => "throughput",
    Latency => "latency",
});

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum StringOrList {
    String(String),
    List(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ChatContent {
    Text(String),
    Parts(Vec<ContentPart>),
    Null,
}

impl Default for ChatContent {
    fn default() -> Self {
        Self::Text(String::new())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ContentPart {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

impl ContentPart {
    pub fn text(text: impl Into<String>) -> Self {
        let mut extra = JsonObject::new();
        extra.insert("text".to_owned(), Value::String(text.into()));
        Self {
            type_: "text".to_owned(),
            extra,
        }
    }

    pub fn image_url(url: impl Into<String>) -> Self {
        let mut image_url = JsonObject::new();
        image_url.insert("url".to_owned(), Value::String(url.into()));
        let mut extra = JsonObject::new();
        extra.insert(
            "image_url".to_owned(),
            Value::Object(image_url.into_iter().collect()),
        );
        Self {
            type_: "image_url".to_owned(),
            extra,
        }
    }

    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage {
    pub role: ChatRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<ChatContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

impl ChatMessage {
    pub fn new(role: ChatRole, content: ChatContent) -> Self {
        Self {
            role,
            content: Some(content),
            name: None,
            tool_call_id: None,
            extra: JsonObject::new(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self::new(ChatRole::System, ChatContent::Text(content.into()))
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new(ChatRole::User, ChatContent::Text(content.into()))
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(ChatRole::Assistant, ChatContent::Text(content.into()))
    }

    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        let mut msg = Self::new(ChatRole::Tool, ChatContent::Text(content.into()));
        msg.tool_call_id = Some(tool_call_id.into());
        msg
    }

    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ProviderPriceLimit {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<String>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ProviderPreferences {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_fallbacks: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_collection: Option<DataCollection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enforce_distillable_text: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_price: Option<ProviderPriceLimit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub only: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_max_latency: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_min_throughput: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantizations: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_parameters: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zdr: Option<bool>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

impl ProviderPreferences {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn require_parameters(mut self, value: bool) -> Self {
        self.require_parameters = Some(value);
        self
    }

    pub fn only(mut self, providers: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.only = Some(providers.into_iter().map(Into::into).collect());
        self
    }

    pub fn order(mut self, providers: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.order = Some(providers.into_iter().map(Into::into).collect());
        self
    }

    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatCompletionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_config: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<String>>,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modalities: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderPreferences>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugins: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ServiceTier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_server_tools_when: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_a: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repetition_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<Value>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

impl ChatCompletionRequest {
    pub fn new(model: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self::from_model_fields(Some(model.into()), None, messages)
    }

    pub fn new_with_models(
        models: impl IntoIterator<Item = impl Into<String>>,
        messages: Vec<ChatMessage>,
    ) -> Self {
        Self::from_model_fields(
            None,
            Some(models.into_iter().map(Into::into).collect()),
            messages,
        )
    }

    fn from_model_fields(
        model: Option<String>,
        models: Option<Vec<String>>,
        messages: Vec<ChatMessage>,
    ) -> Self {
        Self {
            cache_control: None,
            debug: None,
            image_config: None,
            logit_bias: None,
            model,
            models,
            messages,
            temperature: None,
            max_tokens: None,
            max_completion_tokens: None,
            min_p: None,
            modalities: None,
            response_format: None,
            provider: None,
            plugins: None,
            route: None,
            service_tier: None,
            stream: None,
            stream_options: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            reasoning: None,
            reasoning_effort: None,
            stop: None,
            stop_server_tools_when: None,
            top_a: None,
            top_p: None,
            top_k: None,
            top_logprobs: None,
            logprobs: None,
            frequency_penalty: None,
            presence_penalty: None,
            repetition_penalty: None,
            seed: None,
            user: None,
            session_id: None,
            metadata: None,
            trace: None,
            extra: JsonObject::new(),
        }
    }

    pub fn message(mut self, message: ChatMessage) -> Self {
        self.messages.push(message);
        self
    }

    pub fn temperature(mut self, value: f64) -> Self {
        self.temperature = Some(value);
        self
    }

    pub fn max_tokens(mut self, value: u32) -> Self {
        self.max_tokens = Some(value);
        self
    }

    pub fn stream(mut self, value: bool) -> Self {
        self.stream = Some(value);
        self
    }

    pub fn provider(mut self, value: ProviderPreferences) -> Self {
        self.provider = Some(value);
        self
    }

    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Usage {
    #[serde(default)]
    pub prompt_tokens: Option<i64>,
    #[serde(default)]
    pub completion_tokens: Option<i64>,
    #[serde(default)]
    pub total_tokens: Option<i64>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ChatChoice {
    #[serde(default)]
    pub index: Option<i64>,
    #[serde(default)]
    pub finish_reason: Option<FinishReason>,
    #[serde(default)]
    pub message: Option<ChatMessage>,
    #[serde(default)]
    pub delta: Option<Value>,
    #[serde(default)]
    pub logprobs: Option<Value>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ChatCompletionResponse {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub choices: Vec<ChatChoice>,
    #[serde(default)]
    pub created: Option<i64>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub object: Option<String>,
    #[serde(default)]
    pub usage: Option<Usage>,
    #[serde(default)]
    pub system_fingerprint: Option<String>,
    #[serde(default)]
    pub service_tier: Option<ServiceTier>,
    #[serde(default)]
    pub openrouter_metadata: Option<Value>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

pub type ChatStreamChunk = ChatCompletionResponse;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResponsesRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_config: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tool_calls: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modalities: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugins: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ServiceTier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_server_tools_when: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderPreferences>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

impl ResponsesRequest {
    pub fn new(model: impl Into<String>, input: impl Into<Value>) -> Self {
        Self::from_model_fields(Some(model.into()), None, input.into())
    }

    pub fn new_with_models(
        models: impl IntoIterator<Item = impl Into<String>>,
        input: impl Into<Value>,
    ) -> Self {
        Self::from_model_fields(
            None,
            Some(models.into_iter().map(Into::into).collect()),
            input.into(),
        )
    }

    fn from_model_fields(model: Option<String>, models: Option<Vec<String>>, input: Value) -> Self {
        Self {
            background: None,
            cache_control: None,
            debug: None,
            model,
            models,
            input: Some(input),
            instructions: None,
            previous_response_id: None,
            temperature: None,
            frequency_penalty: None,
            image_config: None,
            include: None,
            max_output_tokens: None,
            max_tool_calls: None,
            metadata: None,
            modalities: None,
            parallel_tool_calls: None,
            plugins: None,
            presence_penalty: None,
            prompt: None,
            prompt_cache_key: None,
            reasoning: None,
            route: None,
            safety_identifier: None,
            service_tier: None,
            session_id: None,
            stop_server_tools_when: None,
            text: None,
            tools: None,
            tool_choice: None,
            provider: None,
            stream: None,
            store: None,
            top_k: None,
            top_logprobs: None,
            top_p: None,
            trace: None,
            truncation: None,
            user: None,
            extra: JsonObject::new(),
        }
    }

    pub fn stream(mut self, value: bool) -> Self {
        self.stream = Some(value);
        self
    }

    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ResponsesResponse {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub object: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub output: Option<Value>,
    #[serde(default)]
    pub output_text: Option<String>,
    #[serde(default)]
    pub usage: Option<Usage>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StreamedResponsesEvent {
    #[serde(default, rename = "type")]
    pub type_: Option<String>,
    #[serde(default)]
    pub response: Option<ResponsesResponse>,
    #[serde(default)]
    pub item: Option<Value>,
    #[serde(default)]
    pub delta: Option<Value>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessagesRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_management: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallbacks: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<String>>,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_config: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugins: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderPreferences>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ServiceTier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_server_tools_when: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

impl MessagesRequest {
    pub fn new(model: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self::from_model_fields(Some(model.into()), None, messages)
    }

    pub fn new_with_models(
        models: impl IntoIterator<Item = impl Into<String>>,
        messages: Vec<ChatMessage>,
    ) -> Self {
        Self::from_model_fields(
            None,
            Some(models.into_iter().map(Into::into).collect()),
            messages,
        )
    }

    fn from_model_fields(
        model: Option<String>,
        models: Option<Vec<String>>,
        messages: Vec<ChatMessage>,
    ) -> Self {
        Self {
            cache_control: None,
            context_management: None,
            fallbacks: None,
            model,
            models,
            messages,
            metadata: None,
            max_tokens: None,
            output_config: None,
            plugins: None,
            system: None,
            temperature: None,
            tools: None,
            tool_choice: None,
            thinking: None,
            provider: None,
            route: None,
            service_tier: None,
            session_id: None,
            speed: None,
            stop_sequences: None,
            stop_server_tools_when: None,
            stream: None,
            top_k: None,
            top_p: None,
            trace: None,
            user: None,
            extra: JsonObject::new(),
        }
    }

    pub fn max_tokens(mut self, value: u32) -> Self {
        self.max_tokens = Some(value);
        self
    }

    pub fn stream(mut self, value: bool) -> Self {
        self.stream = Some(value);
        self
    }

    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct MessagesResponse {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default, rename = "type")]
    pub type_: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub content: Option<Value>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub usage: Option<Usage>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

pub type MessagesStreamEvent = MessagesResponse;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmbeddingsRequest {
    pub model: String,
    pub input: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderPreferences>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

impl EmbeddingsRequest {
    pub fn new(model: impl Into<String>, input: impl Into<Value>) -> Self {
        Self {
            model: model.into(),
            input: input.into(),
            dimensions: None,
            encoding_format: None,
            input_type: None,
            provider: None,
            user: None,
            extra: JsonObject::new(),
        }
    }
}

json_object_type!(EmbeddingsResponse);
json_object_type!(EmbeddingModelsResponse);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum RerankDocument {
    Text(String),
    Object(Value),
}

impl From<String> for RerankDocument {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for RerankDocument {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RerankRequest {
    pub model: String,
    pub query: String,
    pub documents: Vec<RerankDocument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderPreferences>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_n: Option<u32>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

impl RerankRequest {
    pub fn new(
        model: impl Into<String>,
        query: impl Into<String>,
        documents: impl IntoIterator<Item = impl Into<RerankDocument>>,
    ) -> Self {
        Self {
            model: model.into(),
            query: query.into(),
            documents: documents.into_iter().map(Into::into).collect(),
            provider: None,
            top_n: None,
            extra: JsonObject::new(),
        }
    }
}

json_object_type!(RerankResponse);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpeechRequest {
    pub model: String,
    pub input: String,
    pub voice: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

impl SpeechRequest {
    pub fn new(
        model: impl Into<String>,
        input: impl Into<String>,
        voice: impl Into<String>,
    ) -> Self {
        Self {
            model: model.into(),
            input: input.into(),
            voice: voice.into(),
            response_format: None,
            speed: None,
            extra: JsonObject::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TranscriptionRequest {
    pub model: String,
    pub input_audio: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderPreferences>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

impl TranscriptionRequest {
    pub fn new(model: impl Into<String>, input_audio: impl Into<Value>) -> Self {
        Self {
            model: model.into(),
            input_audio: input_audio.into(),
            language: None,
            provider: None,
            temperature: None,
            extra: JsonObject::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TranscriptionFileRequest {
    pub model: String,
    pub bytes: Bytes,
    pub file_name: Option<String>,
    pub content_type: Option<String>,
    pub language: Option<String>,
    pub response_format: Option<String>,
    pub temperature: Option<f64>,
}

impl TranscriptionFileRequest {
    pub fn new(model: impl Into<String>, bytes: impl Into<Bytes>) -> Self {
        Self {
            model: model.into(),
            bytes: bytes.into(),
            file_name: None,
            content_type: None,
            language: None,
            response_format: None,
            temperature: None,
        }
    }

    pub fn with_file_name(mut self, value: impl Into<String>) -> Self {
        self.file_name = Some(value.into());
        self
    }

    pub fn with_content_type(mut self, value: impl Into<String>) -> Self {
        self.content_type = Some(value.into());
        self
    }
}

json_object_type!(TranscriptionResponse);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageGenerationRequest {
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_references: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_compression: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderPreferences>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

impl ImageGenerationRequest {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            prompt: None,
            aspect_ratio: None,
            background: None,
            input_references: None,
            n: None,
            output_compression: None,
            output_format: None,
            provider: None,
            quality: None,
            resolution: None,
            seed: None,
            size: None,
            stream: None,
            extra: JsonObject::new(),
        }
    }

    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }
}

json_object_type!(ImageGenerationResponse);
json_object_type!(ImageModelsResponse);
json_object_type!(ImageModelEndpointsResponse);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoGenerationRequest {
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_images: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_audio: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_references: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderPreferences>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

impl VideoGenerationRequest {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            prompt: None,
            aspect_ratio: None,
            callback_url: None,
            duration: None,
            frame_images: None,
            generate_audio: None,
            input_references: None,
            provider: None,
            resolution: None,
            seed: None,
            size: None,
            extra: JsonObject::new(),
        }
    }

    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }
}

json_object_type!(VideoGenerationResponse);
json_object_type!(VideoStatusResponse);
json_object_type!(VideoModelsResponse);

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BinaryResponse {
    #[serde(skip)]
    pub bytes: Bytes,
    pub content_type: Option<String>,
    pub content_disposition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileUploadRequest {
    #[serde(skip)]
    pub bytes: Bytes,
    pub file_name: Option<String>,
    pub content_type: Option<String>,
}

impl FileUploadRequest {
    pub fn new(bytes: impl Into<Bytes>) -> Self {
        Self {
            bytes: bytes.into(),
            file_name: None,
            content_type: None,
        }
    }

    pub fn with_file_name(mut self, value: impl Into<String>) -> Self {
        self.file_name = Some(value.into());
        self
    }

    pub fn with_content_type(mut self, value: impl Into<String>) -> Self {
        self.content_type = Some(value.into());
        self
    }
}

json_object_type!(FileListResponse);
json_object_type!(FileMetadataResponse);
json_object_type!(FileDeleteResponse);
json_object_type!(FileUploadResponse);

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct GenerationResponse {
    #[serde(default)]
    pub data: Option<GenerationData>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct GenerationData {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub total_cost: Option<f64>,
    #[serde(default)]
    pub usage: Option<f64>,
    #[serde(default)]
    pub provider_name: Option<String>,
    #[serde(default)]
    pub latency: Option<f64>,
    #[serde(default)]
    pub tokens_prompt: Option<i64>,
    #[serde(default)]
    pub tokens_completion: Option<i64>,
    #[serde(default)]
    pub finish_reason: Option<String>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

impl GenerationResponse {
    pub fn total_cost(&self) -> Option<f64> {
        self.data.as_ref().and_then(|data| data.total_cost)
    }
}

json_object_type!(GenerationContentResponse);

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct DataResponse<T> {
    #[serde(default)]
    pub data: Option<T>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ListResponse<T> {
    #[serde(default)]
    pub data: Vec<T>,
    #[serde(default)]
    pub total_count: Option<i64>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct DeleteResponse {
    #[serde(default)]
    pub deleted: Option<bool>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ActivityResponse {
    #[serde(default)]
    pub data: Vec<ActivityItem>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ActivityItem {
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub provider_name: Option<String>,
    #[serde(default)]
    pub usage: Option<f64>,
    #[serde(default)]
    pub tokens_prompt: Option<i64>,
    #[serde(default)]
    pub tokens_completion: Option<i64>,
    #[serde(default)]
    pub requests: Option<i64>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct AnalyticsMetaResponse {
    #[serde(default)]
    pub data: Option<AnalyticsMetaData>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct AnalyticsMetaData {
    #[serde(default)]
    pub metrics: Option<Value>,
    #[serde(default)]
    pub dimensions: Option<Value>,
    #[serde(default)]
    pub operators: Option<Value>,
    #[serde(default)]
    pub granularities: Option<Value>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct AnalyticsQueryRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub granularity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_range: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_by: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_limit: Option<u32>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

impl AnalyticsQueryRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct AnalyticsQueryResponse {
    #[serde(default)]
    pub data: Option<AnalyticsQueryData>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct AnalyticsQueryData {
    #[serde(default)]
    pub data: Option<Value>,
    #[serde(default)]
    pub metadata: Option<Value>,
    #[serde(default)]
    pub warnings: Option<Value>,
    #[serde(default, rename = "cachedAt")]
    pub cached_at: Option<String>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}
json_object_type!(AuthKeyExchangeRequest);
json_object_type!(AuthKeyExchangeResponse);
json_object_type!(AuthKeyCodeRequest);
json_object_type!(AuthKeyCodeResponse);
json_object_type!(BenchmarksResponse);
json_object_type!(TaskClassificationResponse);
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct CreditsResponse {
    #[serde(default)]
    pub data: Option<CreditsData>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct CreditsData {
    #[serde(default)]
    pub total_credits: Option<f64>,
    #[serde(default)]
    pub total_usage: Option<f64>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}
json_object_type!(AppRankingsResponse);
json_object_type!(RankingsDailyResponse);
json_object_type!(EndpointsZdrResponse);
pub type CurrentKeyResponse = DataResponse<KeyData>;
pub type KeyListResponse = ListResponse<KeyData>;
pub type KeyResponse = DataResponse<KeyData>;
pub type KeyDeleteResponse = DeleteResponse;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct KeyCreateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_reset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_byok_in_limit: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator_user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct KeyUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_reset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_byok_in_limit: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct KeyCreateResponse {
    #[serde(default)]
    pub data: Option<KeyData>,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

pub type KeyUpdateResponse = DataResponse<KeyData>;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct KeyData {
    #[serde(default)]
    pub hash: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub disabled: Option<bool>,
    #[serde(default)]
    pub limit: Option<f64>,
    #[serde(default)]
    pub limit_reset: Option<String>,
    #[serde(default)]
    pub include_byok_in_limit: Option<bool>,
    #[serde(default)]
    pub usage: Option<f64>,
    #[serde(default)]
    pub usage_limit: Option<f64>,
    #[serde(default)]
    pub byok_usage: Option<f64>,
    #[serde(default)]
    pub byok_usage_limit: Option<f64>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub creator_user_id: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}
json_object_type!(ModelListResponse);
json_object_type!(ModelResponse);
json_object_type!(ModelEndpointsResponse);
json_object_type!(ModelCountResponse);
json_object_type!(UserModelsResponse);
json_object_type!(ProviderListResponse);
pub type ByokListResponse = ListResponse<ByokKey>;
pub type ByokCreateResponse = DataResponse<ByokKey>;
pub type ByokResponse = DataResponse<ByokKey>;
pub type ByokUpdateResponse = DataResponse<ByokKey>;
pub type ByokDeleteResponse = DeleteResponse;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ByokCreateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_fallback: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_models: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_user_ids: Option<Vec<String>>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

pub type ByokUpdateRequest = ByokCreateRequest;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ByokKey {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub disabled: Option<bool>,
    #[serde(default)]
    pub is_fallback: Option<bool>,
    #[serde(default)]
    pub allowed_models: Option<Vec<String>>,
    #[serde(default)]
    pub allowed_api_key_hashes: Option<Vec<String>>,
    #[serde(default)]
    pub allowed_user_ids: Option<Vec<String>>,
    #[serde(default)]
    pub sort_order: Option<i64>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

pub type GuardrailListResponse = ListResponse<Guardrail>;
pub type GuardrailCreateResponse = DataResponse<Guardrail>;
pub type GuardrailResponse = DataResponse<Guardrail>;
pub type GuardrailUpdateResponse = DataResponse<Guardrail>;
pub type GuardrailDeleteResponse = DeleteResponse;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct GuardrailCreateRequest {
    #[serde(flatten)]
    pub fields: GuardrailFields,
}

pub type GuardrailUpdateRequest = GuardrailCreateRequest;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct GuardrailFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_interval: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_models: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignored_models: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_providers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignored_providers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_filters: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_filter_builtins: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enforce_zdr: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enforce_zdr_models: Option<Vec<String>>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Guardrail {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(flatten)]
    pub fields: GuardrailFields,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

pub type KeyAssignmentsResponse = ListResponse<KeyAssignment>;
pub type MemberAssignmentsResponse = ListResponse<MemberAssignment>;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct KeyAssignment {
    #[serde(default)]
    pub key_hash: Option<String>,
    #[serde(default)]
    pub guardrail_id: Option<String>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct MemberAssignment {
    #[serde(default)]
    pub member_user_id: Option<String>,
    #[serde(default)]
    pub guardrail_id: Option<String>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BulkAssignKeysRequest {
    #[serde(default)]
    pub key_hashes: Vec<String>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

pub type BulkUnassignKeysRequest = BulkAssignKeysRequest;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BulkAssignMembersRequest {
    #[serde(default)]
    pub member_user_ids: Vec<String>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

pub type BulkUnassignMembersRequest = BulkAssignMembersRequest;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct AssignmentCountResponse {
    #[serde(default)]
    pub assigned_count: Option<i64>,
    #[serde(default)]
    pub unassigned_count: Option<i64>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

pub type BulkAssignKeysResponse = AssignmentCountResponse;
pub type BulkUnassignKeysResponse = AssignmentCountResponse;
pub type BulkAssignMembersResponse = AssignmentCountResponse;
pub type BulkUnassignMembersResponse = AssignmentCountResponse;

pub type ObservabilityDestinationListResponse = ListResponse<ObservabilityDestination>;
pub type ObservabilityDestinationCreateResponse = DataResponse<ObservabilityDestination>;
pub type ObservabilityDestinationResponse = DataResponse<ObservabilityDestination>;
pub type ObservabilityDestinationUpdateResponse = DataResponse<ObservabilityDestination>;
pub type ObservabilityDestinationDeleteResponse = DeleteResponse;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ObservabilityDestinationCreateRequest {
    #[serde(flatten)]
    pub fields: ObservabilityDestinationFields,
}

pub type ObservabilityDestinationUpdateRequest = ObservabilityDestinationCreateRequest;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ObservabilityDestinationFields {
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub type_: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_hashes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_rules: Option<Value>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ObservabilityDestination {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(flatten)]
    pub fields: ObservabilityDestinationFields,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

pub type OrganizationMembersResponse = ListResponse<OrganizationMember>;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct OrganizationMember {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}
json_object_type!(PresetListResponse);
json_object_type!(PresetResponse);
json_object_type!(PresetCreateFromInferenceResponse);
json_object_type!(PresetVersionListResponse);
json_object_type!(PresetVersionResponse);
pub type WorkspaceListResponse = ListResponse<Workspace>;
pub type WorkspaceCreateResponse = DataResponse<Workspace>;
pub type WorkspaceResponse = DataResponse<Workspace>;
pub type WorkspaceUpdateResponse = DataResponse<Workspace>;
pub type WorkspaceDeleteResponse = DeleteResponse;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct WorkspaceCreateRequest {
    #[serde(flatten)]
    pub fields: WorkspaceFields,
}

pub type WorkspaceUpdateRequest = WorkspaceCreateRequest;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct WorkspaceFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_text_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_image_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_provider_sort: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_logging_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_observability_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub io_logging_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub io_logging_sampling_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub io_logging_api_key_ids: Option<Vec<String>>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Workspace {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(flatten)]
    pub fields: WorkspaceFields,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub created_by: Option<String>,
}

pub type WorkspaceBudgetListResponse = ListResponse<WorkspaceBudget>;
pub type WorkspaceBudgetUpsertResponse = DataResponse<WorkspaceBudget>;
pub type WorkspaceBudgetDeleteResponse = DeleteResponse;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct WorkspaceBudgetUpsertRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_usd: Option<f64>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct WorkspaceBudget {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub limit_usd: Option<f64>,
    #[serde(default)]
    pub reset_interval: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

pub type WorkspaceMemberListResponse = ListResponse<WorkspaceMember>;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct WorkspaceMember {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BulkAddWorkspaceMembersRequest {
    #[serde(default)]
    pub user_ids: Vec<String>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

pub type BulkRemoveWorkspaceMembersRequest = BulkAddWorkspaceMembersRequest;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BulkAddWorkspaceMembersResponse {
    #[serde(default)]
    pub data: Vec<WorkspaceMember>,
    #[serde(default)]
    pub added_count: Option<i64>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BulkRemoveWorkspaceMembersResponse {
    #[serde(default)]
    pub removed_count: Option<i64>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

#[cfg(test)]
mod tests {
    use super::{
        ChatCompletionRequest, ChatMessage, ChatRole, EmbeddingsRequest, ImageGenerationRequest,
        MessagesRequest, ProviderPreferences, RerankDocument, RerankRequest, ResponsesRequest,
        TranscriptionRequest, VideoGenerationRequest,
    };
    use serde_json::json;

    #[test]
    fn unknown_enums_round_trip() {
        let role: ChatRole = serde_json::from_str("\"new_role\"").unwrap();
        assert_eq!(role.as_str(), "new_role");
        assert_eq!(serde_json::to_string(&role).unwrap(), "\"new_role\"");
    }

    #[test]
    fn provider_preferences_serialize_stable_and_extra_fields() {
        let prefs = ProviderPreferences::new()
            .require_parameters(true)
            .only(["openai"])
            .with_field("custom", true);
        let value = serde_json::to_value(prefs).unwrap();
        assert_eq!(value["require_parameters"], true);
        assert_eq!(value["only"][0], "openai");
        assert_eq!(value["custom"], true);
    }

    #[test]
    fn multi_model_request_constructors_serialize_models_without_model() {
        let chat = ChatCompletionRequest::new_with_models(
            ["openai/gpt-4o-mini", "anthropic/claude-haiku"],
            vec![ChatMessage::user("hi")],
        );
        let chat = serde_json::to_value(chat).unwrap();
        assert!(chat.get("model").is_none());
        assert_eq!(chat["models"][0], "openai/gpt-4o-mini");

        let responses = ResponsesRequest::new_with_models(
            ["openai/gpt-4o-mini", "anthropic/claude-haiku"],
            "hi",
        );
        let responses = serde_json::to_value(responses).unwrap();
        assert!(responses.get("model").is_none());
        assert_eq!(responses["models"][1], "anthropic/claude-haiku");

        let messages = MessagesRequest::new_with_models(
            ["anthropic/claude-sonnet", "anthropic/claude-haiku"],
            vec![ChatMessage::user("hi")],
        );
        let messages = serde_json::to_value(messages).unwrap();
        assert!(messages.get("model").is_none());
        assert_eq!(messages["models"][0], "anthropic/claude-sonnet");
    }

    #[test]
    fn expanded_request_fields_serialize_with_openrouter_wire_names() {
        let mut chat =
            ChatCompletionRequest::new("openai/gpt-4o-mini", vec![ChatMessage::user("hello")]);
        chat.cache_control = Some(json!({"type": "ephemeral"}));
        chat.modalities = Some(vec!["text".to_owned(), "image".to_owned()]);
        chat.stop_server_tools_when = Some(json!("first_tool_call"));
        chat.top_a = Some(0.4);
        chat.trace = Some(json!({"enabled": true}));
        let chat = serde_json::to_value(chat).unwrap();
        assert_eq!(chat["cache_control"]["type"], "ephemeral");
        assert_eq!(chat["modalities"][1], "image");
        assert_eq!(chat["stop_server_tools_when"], "first_tool_call");
        assert_eq!(chat["top_a"], 0.4);
        assert_eq!(chat["trace"]["enabled"], true);

        let mut responses = ResponsesRequest::new("openai/gpt-4o-mini", "hello");
        responses.background = Some(true);
        responses.max_tool_calls = Some(3);
        responses.prompt_cache_key = Some("cache-key".to_owned());
        responses.safety_identifier = Some("user-1".to_owned());
        responses.truncation = Some("auto".to_owned());
        let responses = serde_json::to_value(responses).unwrap();
        assert_eq!(responses["background"], true);
        assert_eq!(responses["max_tool_calls"], 3);
        assert_eq!(responses["prompt_cache_key"], "cache-key");
        assert_eq!(responses["safety_identifier"], "user-1");
        assert_eq!(responses["truncation"], "auto");

        let mut messages =
            MessagesRequest::new("anthropic/claude-haiku", vec![ChatMessage::user("hello")]);
        messages.context_management = Some(json!({"clear_function_results": true}));
        messages.stop_sequences = Some(vec!["END".to_owned()]);
        messages.speed = Some(1.1);
        messages.top_p = Some(0.9);
        let messages = serde_json::to_value(messages).unwrap();
        assert_eq!(
            messages["context_management"]["clear_function_results"],
            true
        );
        assert_eq!(messages["stop_sequences"][0], "END");
        assert_eq!(messages["speed"], 1.1);
        assert_eq!(messages["top_p"], 0.9);

        let mut image = ImageGenerationRequest::new("openai/gpt-image-1").prompt("a chair");
        image.aspect_ratio = Some("1:1".to_owned());
        image.n = Some(2);
        image.output_format = Some("png".to_owned());
        image.stream = Some(true);
        let image = serde_json::to_value(image).unwrap();
        assert_eq!(image["aspect_ratio"], "1:1");
        assert_eq!(image["n"], 2);
        assert_eq!(image["output_format"], "png");
        assert_eq!(image["stream"], true);

        let mut video = VideoGenerationRequest::new("provider/video").prompt("waves");
        video.callback_url = Some("https://example.test/callback".to_owned());
        video.generate_audio = Some(false);
        video.frame_images = Some(json!(["image-1"]));
        let video = serde_json::to_value(video).unwrap();
        assert_eq!(video["callback_url"], "https://example.test/callback");
        assert_eq!(video["generate_audio"], false);
        assert_eq!(video["frame_images"][0], "image-1");

        let mut embeddings = EmbeddingsRequest::new("openai/text-embedding-3-small", "hello");
        embeddings.dimensions = Some(128);
        embeddings.encoding_format = Some("float".to_owned());
        embeddings.input_type = Some("query".to_owned());
        let embeddings = serde_json::to_value(embeddings).unwrap();
        assert_eq!(embeddings["dimensions"], 128);
        assert_eq!(embeddings["encoding_format"], "float");
        assert_eq!(embeddings["input_type"], "query");

        let mut rerank = RerankRequest::new(
            "cohere/rerank",
            "query",
            [
                RerankDocument::Text("plain".to_owned()),
                RerankDocument::Object(json!({"text": "rich"})),
            ],
        );
        rerank.top_n = Some(1);
        let rerank = serde_json::to_value(rerank).unwrap();
        assert_eq!(rerank["documents"][0], "plain");
        assert_eq!(rerank["documents"][1]["text"], "rich");
        assert_eq!(rerank["top_n"], 1);

        let mut transcription =
            TranscriptionRequest::new("openai/whisper-1", json!({"data": "base64"}));
        transcription.temperature = Some(0.2);
        let transcription = serde_json::to_value(transcription).unwrap();
        assert_eq!(transcription["temperature"], 0.2);
        assert_eq!(transcription["input_audio"]["data"], "base64");
    }
}
