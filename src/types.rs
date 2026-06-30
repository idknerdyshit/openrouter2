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
    pub response_format: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderPreferences>,
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
            model,
            models,
            messages,
            temperature: None,
            max_tokens: None,
            max_completion_tokens: None,
            response_format: None,
            provider: None,
            stream: None,
            stream_options: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            reasoning: None,
            reasoning_effort: None,
            stop: None,
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
    pub max_output_tokens: Option<u32>,
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
            model,
            models,
            input: Some(input),
            instructions: None,
            previous_response_id: None,
            temperature: None,
            max_output_tokens: None,
            tools: None,
            tool_choice: None,
            provider: None,
            stream: None,
            store: None,
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
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
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
    pub stream: Option<bool>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

impl MessagesRequest {
    pub fn new(model: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self {
            model: model.into(),
            messages,
            max_tokens: None,
            system: None,
            temperature: None,
            tools: None,
            tool_choice: None,
            thinking: None,
            provider: None,
            stream: None,
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
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

impl EmbeddingsRequest {
    pub fn new(model: impl Into<String>, input: impl Into<Value>) -> Self {
        Self {
            model: model.into(),
            input: input.into(),
            extra: JsonObject::new(),
        }
    }
}

json_object_type!(EmbeddingsResponse);
json_object_type!(EmbeddingModelsResponse);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RerankRequest {
    pub model: String,
    pub query: String,
    pub documents: Vec<String>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

impl RerankRequest {
    pub fn new(model: impl Into<String>, query: impl Into<String>, documents: Vec<String>) -> Self {
        Self {
            model: model.into(),
            query: query.into(),
            documents,
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
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

impl TranscriptionRequest {
    pub fn new(model: impl Into<String>, input_audio: impl Into<Value>) -> Self {
        Self {
            model: model.into(),
            input_audio: input_audio.into(),
            language: None,
            extra: JsonObject::new(),
        }
    }
}

json_object_type!(TranscriptionResponse);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageGenerationRequest {
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

impl ImageGenerationRequest {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            prompt: None,
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
    #[serde(default, flatten)]
    pub extra: JsonObject,
}

impl VideoGenerationRequest {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            prompt: None,
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

json_object_type!(ActivityResponse);
json_object_type!(AnalyticsMetaResponse);
json_object_type!(AnalyticsQueryRequest);
json_object_type!(AnalyticsQueryResponse);
json_object_type!(AuthKeyExchangeRequest);
json_object_type!(AuthKeyExchangeResponse);
json_object_type!(AuthKeyCodeRequest);
json_object_type!(AuthKeyCodeResponse);
json_object_type!(BenchmarksResponse);
json_object_type!(TaskClassificationResponse);
json_object_type!(CreditsResponse);
json_object_type!(AppRankingsResponse);
json_object_type!(RankingsDailyResponse);
json_object_type!(EndpointsZdrResponse);
json_object_type!(CurrentKeyResponse);
json_object_type!(KeyListResponse);
json_object_type!(KeyCreateRequest);
json_object_type!(KeyCreateResponse);
json_object_type!(KeyResponse);
json_object_type!(KeyUpdateRequest);
json_object_type!(KeyUpdateResponse);
json_object_type!(KeyDeleteResponse);
json_object_type!(ModelListResponse);
json_object_type!(ModelResponse);
json_object_type!(ModelEndpointsResponse);
json_object_type!(ModelCountResponse);
json_object_type!(UserModelsResponse);
json_object_type!(ProviderListResponse);
json_object_type!(ByokListResponse);
json_object_type!(ByokCreateRequest);
json_object_type!(ByokCreateResponse);
json_object_type!(ByokResponse);
json_object_type!(ByokUpdateRequest);
json_object_type!(ByokUpdateResponse);
json_object_type!(ByokDeleteResponse);
json_object_type!(GuardrailListResponse);
json_object_type!(GuardrailCreateRequest);
json_object_type!(GuardrailCreateResponse);
json_object_type!(GuardrailResponse);
json_object_type!(GuardrailUpdateRequest);
json_object_type!(GuardrailUpdateResponse);
json_object_type!(GuardrailDeleteResponse);
json_object_type!(KeyAssignmentsResponse);
json_object_type!(MemberAssignmentsResponse);
json_object_type!(BulkAssignKeysRequest);
json_object_type!(BulkAssignKeysResponse);
json_object_type!(BulkUnassignKeysRequest);
json_object_type!(BulkUnassignKeysResponse);
json_object_type!(BulkAssignMembersRequest);
json_object_type!(BulkAssignMembersResponse);
json_object_type!(BulkUnassignMembersRequest);
json_object_type!(BulkUnassignMembersResponse);
json_object_type!(ObservabilityDestinationListResponse);
json_object_type!(ObservabilityDestinationCreateRequest);
json_object_type!(ObservabilityDestinationCreateResponse);
json_object_type!(ObservabilityDestinationResponse);
json_object_type!(ObservabilityDestinationUpdateRequest);
json_object_type!(ObservabilityDestinationUpdateResponse);
json_object_type!(ObservabilityDestinationDeleteResponse);
json_object_type!(OrganizationMembersResponse);
json_object_type!(PresetListResponse);
json_object_type!(PresetResponse);
json_object_type!(PresetCreateFromInferenceResponse);
json_object_type!(PresetVersionListResponse);
json_object_type!(PresetVersionResponse);
json_object_type!(WorkspaceListResponse);
json_object_type!(WorkspaceCreateRequest);
json_object_type!(WorkspaceCreateResponse);
json_object_type!(WorkspaceResponse);
json_object_type!(WorkspaceUpdateRequest);
json_object_type!(WorkspaceUpdateResponse);
json_object_type!(WorkspaceDeleteResponse);
json_object_type!(WorkspaceBudgetListResponse);
json_object_type!(WorkspaceBudgetUpsertRequest);
json_object_type!(WorkspaceBudgetUpsertResponse);
json_object_type!(WorkspaceBudgetDeleteResponse);
json_object_type!(BulkAddWorkspaceMembersRequest);
json_object_type!(BulkAddWorkspaceMembersResponse);
json_object_type!(BulkRemoveWorkspaceMembersRequest);
json_object_type!(BulkRemoveWorkspaceMembersResponse);

#[cfg(test)]
mod tests {
    use super::{
        ChatCompletionRequest, ChatMessage, ChatRole, ProviderPreferences, ResponsesRequest,
    };

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
    }
}
