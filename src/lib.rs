#![cfg_attr(
    not(any(feature = "async", feature = "blocking")),
    allow(
        dead_code,
        unused_imports,
        unused_macros,
        reason = "support modules are only consumed when a client feature is enabled"
    )
)]

//! Typed clients for the OpenRouter API.
//!
//! `openrouter2` exposes route-complete async and blocking clients for the
//! current non-deprecated OpenRouter API surface. Async support is enabled by
//! default through `AsyncOpenRouterClient`. Enable the `blocking` Cargo feature
//! for `BlockingOpenRouterClient`.
//!
//! Client values can store an optional default `ApiKey` alongside the injected
//! HTTP client and normalized base URL. Per-call `RequestOptions` can override
//! that key or explicitly suppress authentication for no-auth routes.

mod auth;
mod client_routes;
mod error;
mod observability;
mod options;
mod retry;
mod routes;
mod spec;
mod transport;
pub mod types;

#[cfg(feature = "async")]
mod async_client;
#[cfg(feature = "blocking")]
mod blocking_client;
#[cfg(any(feature = "async", feature = "blocking"))]
pub mod streaming;
#[cfg(any(feature = "async", feature = "blocking"))]
pub use streaming::{SseMessage, StreamApiError};

#[cfg(feature = "async")]
pub use async_client::{AsyncOpenRouterClient, AsyncOpenRouterClientBuilder};
pub use auth::ApiKey;
#[cfg(feature = "blocking")]
pub use blocking_client::{BlockingOpenRouterClient, BlockingOpenRouterClientBuilder};
pub use error::{ApiError, OpenRouterApiError, OpenRouterError};
pub use options::{RequestAuth, RequestOptions};
pub use retry::RetryPolicy;
pub use routes::{HttpMethod, MultipartFile, RawJsonRequest, RawMultipartRequest};
pub use spec::{NON_DEPRECATED_ROUTES, RouteSpec, SPEC_SNAPSHOT_DATE};
pub use transport::{
    DEFAULT_BASE_URL, IntoQueryParams, endpoint_url_from_base, normalize_base_url,
    normalize_unchecked_base_url,
};
pub use types::*;
