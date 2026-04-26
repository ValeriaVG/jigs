# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.1] - 2026-04-26

Initial release. APIs are unstable and expected to change.

### Added
- `jigs-core`: `Request`, `Response`, `Branch`, `Jig` trait, `.then(...)` chaining, and async chaining via `Pending` / `Step`.
- `jigs-macros`: `#[jig]` attribute macro for sync and `async fn` jigs. Optional `trace` feature instruments each call site.
- `jigs-trace`: thread-local execution buffer with `enter` / `exit` / `take` and a `Status` re-export.
- `jigs`: umbrella crate that re-exports the runtime and macro, plus `trace` re-export under the `trace` feature.
- Examples: `hello` (sync), `async`, and a minimal `http` server.
