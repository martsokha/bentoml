# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Breaking:** renamed `EndpointResponse` to `EndpointReply`.
- **Breaking:** `Endpoint::call` now returns an `EndpointReply` (read it as `.json::<R>()` /
  `.bytes()` / `.text()`) instead of deserializing to `R` directly. The old
  `call_json` is gone — `call` *is* the JSON-body method, aligning the `call` /
  `call_bytes` / `call_multipart` family with `submit` / `submit_bytes` /
  `submit_multipart`. Update `call(&p).await?` to `call(&p).await?.json().await?`, or
  use the new `invoke` shorthand.
- **Breaking:** streaming is now a reader on `EndpointReply` rather than a separate
  `Endpoint::stream` method. Call any input method and stream its response:
  `endpoint.call(&p).await?.stream()` (also `call_bytes` / `call_multipart`),
  closing the gap where only a JSON-bodied request could be streamed.
- **Breaking:** renamed `TaskHandle::get` to `TaskHandle::json`, matching the `EndpointReply`
  readers.

### Added

- `Endpoint::invoke`, a JSON-in/JSON-out shorthand that deserializes the response in
  one step (`invoke(&p)` == `call(&p).await?.json().await?`).
- `Endpoint::submit_bytes` and `Endpoint::submit_multipart` for raw-binary and
  `multipart/form-data` task inputs, mirroring `call_bytes` / `call_multipart` (a
  `@bentoml.task` accepts the same inputs as a regular endpoint).
- `TaskHandle::bytes` and `TaskHandle::text` read a completed task's result as raw
  bytes (binary/file output) or UTF-8 text, alongside the existing `json`.

## [0.4.0] - 2026-05-27

### Changed

- **Breaking:** removed the `Readiness` trait; `is_ready`, `is_live`, and
  `wait_until_ready` are now inherent methods on `Client`. Drop the
  `use bentoml::prelude::Readiness;` import (or the prelude glob covers it); the
  calls themselves are unchanged.
- **Breaking:** removed the `Tasks` trait; `submit` is now an inherent method on
  `Endpoint`. Drop the `use bentoml::prelude::Tasks;` import; the call is unchanged.
- **Breaking:** removed the `Streaming` trait; `stream` is now an inherent method on
  `Endpoint` (behind the `stream` feature). Drop the `use bentoml::prelude::Streaming;`
  import; the call is unchanged.
- **Breaking:** consolidated the `Error` enum (9 → 6 variants): `InvalidUrl`,
  `InvalidHeader`, and `InvalidMultipart` fold into one `InvalidRequest`, and the
  build-time `reqwest::Error` now maps into `Transport` (the former `Middleware`
  variant is gone — `Transport` covers all network/transport failures). Added
  `Error::status() -> Option<u16>`; `InvalidRequest`/`Decode` now carry a source for
  error-chain reporting.
- **Breaking:** removed the `Files` trait. Endpoint requests are now inherent methods
  on `Endpoint` that take the request body and return an `EndpointResponse`, so input
  and output encodings are chosen independently: `call_json` / `call_bytes` /
  `call_multipart` return an `EndpointResponse` read via `.json::<R>()`, `.bytes()`,
  or `.text()`. `call(&payload) -> R` remains as the JSON-in/JSON-out shorthand. This
  closes the previous gaps (e.g. raw-in/bytes-out, multipart-in/bytes-out).
- **Breaking:** the per-request timeout now defaults to none (matching reqwest),
  rather than 30s, so long-running inference, tasks, and streaming aren't cut off.
  Set one with `ClientBuilder::with_timeout`.
- **Breaking:** removed the public `DEFAULT_BASE_URL`, `DEFAULT_TIMEOUT`, and
  `DEFAULT_MAX_RETRIES` constants; the defaults are now documented on the
  corresponding `ClientBuilder` setters.

### Added

- `EndpointResponse`, returned by the `call_*` methods, with `.json::<R>()`,
  `.bytes()`, `.text()`, `.status()`, and `.into_inner()`.
- `ByteStream::json::<T>()` yields one deserialized `T` per JSON value, parsing the
  concatenated-JSON wire format BentoML uses for `Generator[Model]` endpoints
  (buffered across chunk boundaries).
- A `multipart` module: the `Multipart` builder (`field` JSON-encodes a parameter
  into its own form field, matching BentoML's per-parameter encoding) and `Part`
  (bytes plus optional file name and MIME type), so callers keep typed values instead
  of hand-assembling a form.
- With the `tracing` feature, a caller-set `x-request-id` (via `with_request_id` or
  `with_header`) is recorded as a `request_id` field on request spans, across the
  endpoint methods and the task lifecycle.

## [0.3.0] - 2026-05-25

### Changed

- **Breaking:** endpoint calls are now made through an `Endpoint` handle obtained
  from `Client::endpoint(route)`, rather than passing the route to each `Client`
  method. The generic `call` and the `Tasks`/`Files`/`Streaming` traits move to
  `Endpoint`, dropping the route argument; `Readiness` health checks remain on
  `Client`. For example, `client.call("summarize", &req)` becomes
  `client.endpoint("summarize").call(&req)`, and `client.submit("generate", &req)`
  becomes `client.endpoint("generate").submit(&req)`.

### Added

- `Client::endpoint(route)` returning a cheap, cloneable `Endpoint` handle.
- Per-call headers on `Endpoint`: `with_header`, `with_headers`, and a
  `with_request_id` helper, applied to every operation on the handle.
- `ClientBuilder::with_user_agent` and `ClientBuilder::with_authorization`
  convenience header helpers.

## [0.2.0] - 2026-05-25

Wire-protocol corrections verified against the BentoML server source. The
`TaskStatus` change is breaking.

### Fixed

- `TaskStatus` variants now match BentoML's `ResultStatus` wire values:
  `in_progress`, `completed`, `failed`, and `canceled` (were `running`,
  `success`, `failure`, `cancelled`), so task status responses deserialize
  correctly.

### Changed

- **Breaking:** renamed `TaskStatus` variants to `Pending`, `InProgress`,
  `Completed`, `Failed`, `Canceled`.
- `TaskInfo` now exposes the `created_at`, `executed_at`, and `completed_at`
  timestamps. The first two are naive `jiff::civil::DateTime`; `completed_at`
  is a timezone-aware `jiff::Timestamp`, matching how BentoML reports each.
- Non-success responses now parse BentoML's `{"error", "detail"}` JSON envelope:
  `Error::Service` carries the `error` message and optional structured `detail`.
- `TaskHandle::get` now checks the task status first and returns
  `Error::TaskNotComplete` unless the task has `Completed`.

### Added

- `ClientBuilder::with_header` for custom headers sent on every request.
- `ByteStream::text` and `ByteStream::lines` adapters for streaming endpoints
  (UTF-8 chunks and newline-delimited records, e.g. JSONL).
- `#[tracing::instrument]` on the `TaskHandle` methods (behind `tracing`).

## [0.1.0] - 2026-05-25

Initial release.

### Added

- `Client` with a `ClientBuilder` (base URL, bearer token, timeout, retries).
- Generic `Client::call` for invoking arbitrary JSON service endpoints.
- Retries for transient failures via `reqwest-middleware` (exponential backoff).
- `Readiness` trait: `is_ready` (`/readyz`), `is_live` (`/livez`), and a
  runtime-agnostic `wait_until_ready`.
- `Tasks` trait for async task queues (`@bentoml.task`): `submit` returns a
  `TaskHandle` with `status` / `get` / `retry` / `cancel`; `TaskStatus`, `TaskInfo`.
- `Files` trait: `call_multipart` (file inputs), `call_raw` (raw-binary root input),
  and `call_bytes` (binary responses).
- `Streaming` trait (feature `stream`): `stream` returns a `ByteStream`.
- `rustls-tls` (default), `native-tls`, `stream`, and `tracing` feature flags.

[unreleased]: https://github.com/martsokha/bentoml/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/martsokha/bentoml/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/martsokha/bentoml/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/martsokha/bentoml/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/martsokha/bentoml/releases/tag/v0.1.0
