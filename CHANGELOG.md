# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial scaffolding for the async BentoML client.
- `Client` with a `ClientBuilder` (base URL, bearer token, timeout).
- Generic `Client::call` for invoking arbitrary service endpoints.
- `Client::is_ready` health check against `/readyz`.
- `rustls-tls` (default), `native-tls`, and `tracing` feature flags.
