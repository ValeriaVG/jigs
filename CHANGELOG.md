# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.1] - 2026-04-26

Initial release. APIs are unstable and expected to change.

### Added
- `jigs-core`: `Request`, `Response`, `Branch`, `Jig` trait, `.then(...)` chaining, and async chaining via `Pending` / `Step`.

## 0.3.0

- `Request` and `Response` are now traits instead of generic wrapper structs. Users define their own domain types and derive the traits, eliminating `r.0` boilerplate.
- `#[derive(Request)]` on structs with one field or `#[req(field = "name")]`.
- `#[derive(Response)]` on structs with one `Result<T, String>` field, two fields, or enums with two variants.
- `Branch<Request, Response>` is now a fully generic enum over any types implementing the `Request` and `Response` traits.
- `.then()` is a trait method on `Request` and `Response`, so custom types chain natively.
- `fork!` extracts payload via `Request::payload()` so it works with any `impl Request`.
- `Response` trait provides default impls for `is_err` and `from_result`.
- `Response::error_msg` added as a non-consuming error accessor.
- `Status::ok` renamed to `Status::succeeded` to avoid collision with `Response::ok` on custom types.
- `Step` and `Status` traits are now implemented generically for any `Branch<REQ, RESP>` where `REQ: Request` and `RESP: Response`.
- `impl_request!` and `impl_response!` macros remain as fallbacks for unsupported derive cases.
- Updated all examples to use derived custom types instead of the old `Request<T>` / `Response<T>` wrappers.

### Breaking changes
- `Request<T>` and `Response<T>` wrapper structs removed. Migrate existing code by defining custom types and deriving `Request`/`Response`.
- `Branch<ReqPayload, RespPayload>` now requires `Branch<RequestType, ResponseType>`.
- `Status::ok` renamed to `Status::succeeded`.

### Deprecated
- Nothing; traits and derives fully replace old wrappers.
