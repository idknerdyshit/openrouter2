# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.3.0] - 2026-06-30

### Added

- Explicit unchecked custom-base constructors and normalizers for local test
  servers and trusted proxies.
- Raw multipart query builder support.
- Shared client route declarations and route-operation coverage checks.

### Changed

- Tightened default base URL and endpoint path validation to require HTTPS
  OpenRouter bases and relative API paths.
- Reworked request construction to validate endpoints before tracing and
  reduced async/blocking route-list duplication.
- Removed `Default` from request types with required fields and added
  multi-model request constructors.
- Updated README guidance for blocking-only and custom-base usage.
- Expanded `.gitignore` with standard Rust, editor, environment, coverage,
  profiling, and OS metadata entries.

### Fixed

- Redacted response error metadata more narrowly by allowlisting safe headers.
- Removed fragments from URL/path redaction and covered absolute raw-path
  rejection before send.
- Avoided extra multipart upload buffer copies.
- Avoided unnecessary JSON cloning on typed SSE events.

### Security

- Prevented raw request paths from escaping the configured base URL and leaking
  bearer tokens.
- Restricted default constructors to trusted HTTPS OpenRouter base URLs.

## [0.2.1] - 2026-06-28

### Added

- `tracing` events for request lifecycle observability with secret redaction.

### Changed

- Redacted sensitive response headers, API error bodies, and transport error
  URLs before they are exposed through error metadata.

## [0.2.0] - 2026-06-28

### Added

- Breaking full-route redesign for the current non-deprecated OpenRouter API
  surface.
- `AsyncOpenRouterClient` behind the default `async` feature.
- `BlockingOpenRouterClient` behind the optional `blocking` feature.
- Shared typed request/response shells, unknown-preserving enums, raw extras,
  per-request options, binary responses, file uploads, and raw request escape
  hatches.
- Typed SSE streaming for chat, responses, and messages.
- Route snapshot for the OpenRouter OpenAPI spec dated 2026-06-28.

### Changed

- Renamed the primary chat method to `create_chat_completion`.
- Replaced cost-only generation lookup with typed generation metadata/content
  APIs plus a `generation_cost` convenience helper.

## [0.1.0] - 2026-06-28

### Added

- Initial standalone `openrouter2` crate with chat completions and generation
  cost lookup support.
