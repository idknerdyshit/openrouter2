//! Typed clients for the OpenRouter API.
//!
//! `openrouter2` exposes route-complete async and blocking clients for the
//! current non-deprecated OpenRouter API surface. Async support is enabled by
//! default through [`AsyncOpenRouterClient`]. Enable the `blocking` Cargo feature
//! for [`BlockingOpenRouterClient`].
//!
//! API keys are always passed per call. Client values only store the injected
//! HTTP client and normalized base URL.

mod error;
mod observability;
mod options;
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

#[cfg(feature = "async")]
pub use async_client::AsyncOpenRouterClient;
#[cfg(feature = "blocking")]
pub use blocking_client::BlockingOpenRouterClient;
pub use error::{ApiError, OpenRouterApiError, OpenRouterError};
pub use options::RequestOptions;
pub use routes::{HttpMethod, MultipartFile, RawJsonRequest, RawMultipartRequest};
pub use spec::{NON_DEPRECATED_ROUTES, RouteSpec, SPEC_SNAPSHOT_DATE};
pub use transport::{DEFAULT_BASE_URL, endpoint_url_from_base, normalize_base_url};
pub use types::*;
