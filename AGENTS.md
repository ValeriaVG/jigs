# jigs

A Rust pipeline framework. Functions annotated with `#[jig]` compose into a compile-time graph via `.then()` chaining.

## Core Types

- `Request` trait — inbound payload; advanced via `.then(jig)`. Implemented by user-defined types via `#[derive(Request)]`.
- `Response` trait — outbound result; errors always flow through response-side jigs (finalizers always run). Implemented by user-defined types via `#[derive(Response)]`.
- `Branch<Req, Resp>` — `Continue(Req)` or `Done(Resp)`; guards that short-circuit. Requires `Req: Request` and `Resp: Response`.
- `Pending<F>` — wraps async futures; `#[jig]` on `async fn` rewrites the signature to return this.

## Four Jig Kinds

| Kind | Signature | What `.then()` accepts it |
|------|-----------|--------------------------|
| Enrich | `impl Request -> impl Request` | After `Request`, after `Branch::Continue` |
| Handler | `impl Request -> impl Response` | After `Request`, after `Branch::Continue` |
| Guard | `impl Request -> Branch<NewReq, Resp>` | After `Request`, after `Branch::Continue` |
| Post-process | `impl Response -> impl Response` | After `Response` (always runs, even on errors) |

## Chaining Rules

- `RequestType.then(jig)` — jig can return `Request`, `Response`, or `Branch`
- `ResponseType.then(jig)` — jig must return `Response<U>`; always runs, even on errors
- `Branch<Req, Resp>.then(jig)` — if `Continue`, runs jig and merges; if `Done`, propagates without running jig
  - jig returns `Request<New>` -> result is `Branch<New, Resp>`
  - jig returns `Response` -> result is `Response` (both branches collapse)
  - jig returns `Branch` -> nested branches flatten

## Macros

### `#[jig]`
Annotates a function. Generates:
- A `__Jig_<fn_name>` marker struct implementing `JigDef` with metadata
- For async fns: rewrites `async fn foo(...) -> T` to `fn foo(...) -> Pending<impl Future<Output = T>>`

### `jigs!(entry_fn)`
Must be placed in the same module as the entry function. Generates `all_jigs()` and `find_jig()`.

### `fork!(req, arm1, arm2, ...)`
Pattern-matching dispatch. Expands to chained `if pred { jig(req) } else { ... }`. `_ => default` arm runs last.

```rust
fork!(req,
    |r: &Raw| r.path.starts_with("/auth/") => features::auth::auth,
    |r: &Raw| r.path.starts_with("/todos") => features::todos::todos,
    _ => not_found,
)
```

### Derive Macros

#### `#[derive(Request)]`
Derives the `Request` trait for a struct. Supports:
- Single-field tuple structs: `struct MyReq(MyPayload)`
- Single-field named structs: `struct MyReq { payload: MyPayload }`
- Multi-field structs with `#[req(field = "field_name")]` to specify the payload field

#### `#[derive(Response)]`
Derives the `Response` trait for a struct or enum. Supports:
- Single-field tuple structs wrapping `Result<T, String>`: `struct MyResp(Result<String, String>)`
- Two-field structs with `#[resp(ok)]` and `#[resp(err)]` attributes
- Enums with exactly two variants, marked with `#[resp(ok)]` and `#[resp(err)]`

## Conventions

- **Module-qualified paths**: When a `#[jig]` fn references a jig from another module, use `features::auth::authenticate`, not a direct import. Same-module jigs can be referenced by name directly. This ensures the `__Jig_<name>` marker struct is visible to macro-generated collection code.
- **fork! arms**: Keep each predicate and handler together. Inline predicates as closures, or place a small named predicate fn immediately above its handler. Don't separate predicates from handlers.
- **Files under 200 LOC, functions under 50 lines**
- **No comments** unless purpose can't be derived from naming
- **No emojis, em dashes, or LLM telltales**

## Workflow

1. Write an interface or type first
2. Write a test for the change you want to make
3. Write the implementation

## Error Handling

- `Response::err("message")` for pipeline errors; errors are always `String`
- `ResponseType.then(jig)` always runs, even on errors—finalizers always see the outcome
- Jigs that should skip on error must check `Response::is_ok()` themselves
- Domain errors (400, 401, etc.) go inside `Response::ok(HttpResponse::bad_request(...))`; `Response::err` is for internal/unexpected failures
- Guards short-circuit with `Branch::Done(Response::err(...))` or `Branch::Done(Response::ok(...))` for successful short-circuits (e.g., cache hits)

## Test Patterns

- Direct `.then()` chains with assertions on the result
- `Branch::Done` short-circuits: verify downstream jigs never run by asserting they would panic
- `fork!` tests: verify first-match semantics and fallthrough
- Async: use `block_on` helper (see `jigs-core/src/tests.rs`)