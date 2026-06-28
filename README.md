# openrouter2

`openrouter2` is a small async Rust client for the OpenRouter API. It currently
wraps the chat completions endpoint and the generation cost lookup endpoint.

The client stores only an injected `reqwest::Client` and normalized base URL.
API keys are passed per call, which makes one shared HTTP client usable across
many accounts without storing credentials in the client value.

## Install

```toml
[dependencies]
openrouter2 = "0.1"
reqwest = { version = "0.13", default-features = false, features = ["json", "rustls"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Usage

```rust
use openrouter2::{
    ChatMessage, ChatRequest, DEFAULT_BASE_URL, OpenRouterClient,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let http = reqwest::Client::new();
    let client = OpenRouterClient::try_new(http, DEFAULT_BASE_URL)?;

    let response = client
        .chat_completion(
            "sk-or-v1-...",
            ChatRequest {
                model: "openai/gpt-4o-mini".to_owned(),
                messages: vec![ChatMessage::user("Write one friendly sentence.")],
                temperature: Some(0.2),
                max_tokens: Some(64),
                response_format: None,
                provider: None,
            },
        )
        .await?;

    println!("{}", response.content);
    Ok(())
}
```

## Base URL

The default base URL is `https://openrouter.ai/api/v1`. To point at a proxy,
test server, or future OpenRouter-compatible base, pass a different URL to
`OpenRouterClient::try_new`.

The base URL is normalized with a trailing slash, must use `http` or `https`,
and must not include a query string or fragment.

## API Surface

- `OpenRouterClient::chat_completion` calls `POST /chat/completions`.
- `OpenRouterClient::generation_cost` calls `GET /generation?id=...`.
- `ChatRequest::response_format` and `ProviderPreferences` support structured
  output options without forcing a larger request model.
- `OpenRouterError::Api` includes the HTTP status and a truncated response body
  for diagnostics.

## MSRV

The minimum supported Rust version is 1.88.
