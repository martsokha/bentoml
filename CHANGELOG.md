# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Breaking:** removed the `Readiness` trait; `is_ready`, `is_live`, and
  `wait_until_ready` are now inherent methods on `Client`. Drop the
  `use bentoml::prelude::Readiness;` import (or the prelude glob covers it); the
  calls themselves are unchanged.
- **Breaking:** removed the `Tasks` trait; `submit` is now an inherent method on
  `Endpoint`. Drop the `use bentoml::prelude::Tasks;` import; the call is unchanged.
- **Breaking:** removed the `Files` trait. Endpoint requests are now inherent methods
  on `Endpoint` that take the request body and return a `Response`, so input and
  output encodings are chosen independently: `call_json` / `call_bytes` /
  `call_multipart` return a `Response` read via `.json::<R>()`, `.bytes()`, or
  `.text()`. `call(&payload) -> R` remains as the JSON-in/JSON-out shorthand. This
  closes the previous gaps (e.g. raw-in/bytes-out, multipart-in/bytes-out).

### Added

- `Response`, returned by the `call_*` methods, with `.json::<R>()`, `.bytes()`, and
  `.text()` readers.
- `ByteStream::json::<T>()` yields one deserialized `T` per JSON value, parsing the
  concatenated-JSON wire format BentoML uses for `Generator[Model]` endpoints
  (buffered across chunk boundaries).
- `Multipart` builder for file/image endpoints: `field` JSON-encodes a parameter into
  its own form field (matching BentoML's per-parameter encoding) and `part` adds a
  file part, so callers keep typed values instead of hand-assembling a form.

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

[unreleased]: https://github.com/martsokha/bentoml/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/martsokha/bentoml/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/martsokha/bentoml/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/martsokha/bentoml/releases/tag/v0.1.0
