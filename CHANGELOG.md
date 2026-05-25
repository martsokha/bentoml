# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
