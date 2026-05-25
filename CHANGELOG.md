# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[unreleased]: https://github.com/martsokha/bentoml/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/martsokha/bentoml/releases/tag/v0.1.0
