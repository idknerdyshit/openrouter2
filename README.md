# openrouter2

`openrouter2` is a typed Rust client for the OpenRouter API. Version `0.4`
targets the full current non-deprecated route set from the OpenRouter OpenAPI
spec snapshot dated `2026-07-08`.

Clients can store an optional default API key. Keys are wrapped in a redacted
zeroizing type, and individual calls can override or suppress auth through
`RequestOptions`.

## Install

```toml
[dependencies]
openrouter2 = "0.4"
reqwest = { version = "0.13", default-features = false, features = ["json", "rustls"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Async support is enabled by default. To enable the blocking client in addition
to the default async client:

```toml
openrouter2 = { version = "0.4", features = ["blocking"] }
```

To build with only the blocking client:

```toml
openrouter2 = { version = "0.4", default-features = false, features = ["blocking"] }
```

## Async Usage

```rust
use openrouter2::{
    AsyncOpenRouterClient, ChatCompletionRequest, ChatMessage, DEFAULT_BASE_URL,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AsyncOpenRouterClient::try_new(reqwest::Client::new(), DEFAULT_BASE_URL)?;
    let client = client.with_api_key("sk-or-v1-...");

    let response = client
        .create_chat_completion(
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
    let client = client.with_api_key("sk-or-v1-...");

    let response = client.create_chat_completion(
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
- Optional stored `ApiKey` with redacted `Debug` output and zeroization on drop.
- Route-complete flat methods for all non-deprecated OpenRouter routes.
- Typed request/response structs for high-value inference and management APIs,
  with raw extras for newly added fields.
- Typed query structs for paginated and filterable list APIs.
- Unknown-preserving string enums and flattened raw `serde_json::Value` extras.
- Typed SSE streaming for chat, responses, and messages.
- Raw JSON, binary, and multipart escape hatches for new API fields/routes.
- `RequestOptions` for per-call headers such as `HTTP-Referer`, `X-Title`,
  `X-Session-Id`, custom headers, API-key override, and explicit no-auth calls.
- First-class multipart speech-to-text helper via `TranscriptionFileRequest`.

`try_new` accepts HTTPS OpenRouter base URLs. Local test servers and trusted
proxies must opt in explicitly with `try_new_unchecked_base_url`.

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

## Examples

Runnable examples live in `examples/`:

- `stored_key_chat.rs` sends a chat completion with a stored client key.
- `transcribe_file.rs` submits multipart audio transcription.
- `workspace_members.rs` lists workspace members with typed pagination.
- `per_request_key_override.rs` overrides the stored key for one request.

## MSRV

The minimum supported Rust version is 1.88.
