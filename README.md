# openrouter2

`openrouter2` is a typed Rust client for the OpenRouter API. Version `0.2`
targets the full current non-deprecated route set from the OpenRouter OpenAPI
spec snapshot dated `2026-06-28`.

The crate keeps API keys out of client state. Pass the key per authenticated
call; the client stores only the injected HTTP client and normalized base URL.

## Install

```toml
[dependencies]
openrouter2 = "0.2"
reqwest = { version = "0.13", default-features = false, features = ["json", "rustls"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Async support is enabled by default. For a blocking client:

```toml
openrouter2 = { version = "0.2", features = ["blocking"] }
```

For blocking-only builds:

```toml
openrouter2 = { version = "0.2", default-features = false, features = ["blocking"] }
```

## Async Usage

```rust
use openrouter2::{
    AsyncOpenRouterClient, ChatCompletionRequest, ChatMessage, DEFAULT_BASE_URL,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AsyncOpenRouterClient::try_new(reqwest::Client::new(), DEFAULT_BASE_URL)?;

    let response = client
        .create_chat_completion(
            "sk-or-v1-...",
            ChatCompletionRequest::new(
                "openai/gpt-4o-mini",
                vec![ChatMessage::user("Write one friendly sentence.")],
            )
            .temperature(0.2)
            .max_tokens(64),
        )
        .await?;

    println!("{response:#?}");
    Ok(())
}
```

## Blocking Usage

```rust
use openrouter2::{
    BlockingOpenRouterClient, ChatCompletionRequest, ChatMessage, DEFAULT_BASE_URL,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client =
        BlockingOpenRouterClient::try_new(reqwest::blocking::Client::new(), DEFAULT_BASE_URL)?;

    let response = client.create_chat_completion(
        "sk-or-v1-...",
        ChatCompletionRequest::new(
            "openai/gpt-4o-mini",
            vec![ChatMessage::user("Write one friendly sentence.")],
        ),
    )?;

    println!("{response:#?}");
    Ok(())
}
```

## API Surface

- Async client: `AsyncOpenRouterClient` behind default feature `async`.
- Blocking client: `BlockingOpenRouterClient` behind feature `blocking`.
- Route-complete flat methods for all non-deprecated OpenRouter routes.
- Typed request/response shells with builder helpers for common fields.
- Unknown-preserving string enums and flattened raw `serde_json::Value` extras.
- Typed SSE streaming for chat, responses, and messages.
- Raw JSON, binary, and multipart escape hatches for new API fields/routes.
- `RequestOptions` for per-call headers such as `HTTP-Referer`, `X-Title`,
  `X-Session-Id`, and custom headers.

## Observability

The crate emits `tracing` events for HTTP request start, completion, non-2xx
responses, and transport errors. It does not install or configure a tracing
subscriber; applications remain in control of where those events go.

Trace fields include method, route template, status, elapsed milliseconds,
request id when present, query count, and whether the request was authenticated.
Request and response bodies are not logged. Bearer tokens, OpenRouter-style
`sk-...` keys, credentials, query values, and sensitive headers such as
`Authorization`, `Cookie`, `Set-Cookie`, token, password, secret, and API-key
fields are redacted before they reach tracing or API error metadata.

Deprecated OpenRouter operations are intentionally skipped. The deprecated
Coinbase credits endpoint is not modeled.

## MSRV

The minimum supported Rust version is 1.88.
