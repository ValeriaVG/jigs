# jigs

Is a Rust request -> response processing framework that allows user to write something like:
```rust
#[jig]
fn handle(request: Request) -> Response {
    request
        .then(log_incoming)
        .then(set_auth_state)
        .then(route_to_feature)
        .then(log_response)
}
```
And get a compile time graph of "jigs" as any element passed to `then` is a jig of its own.

There are 4 types of jigs:
- Request -> Request
- Request -> Response
- Response -> Response
- Response -> Branch that either returns a response or continue passing the request

## Guidelines
- Write an interface or a type first
- Then write a test for the change you want to make
- Only then write implementation itself
- Keep files under 200 LOC and functions under 50
- Use comments only when the purpose of code can't be derived from naming
- Ditch emojis, emdashes and other annoying LLM telltales
- In `fork!`, keep each arm's predicate and handler together: inline the predicate as a closure, or put a small named predicate fn immediately above its handler. Don't separate predicates from handlers and don't extract shared multi-route predicate helpers.
- When a `#[jig]` function references a jig from another module, use the module-qualified path (`features::auth::authenticate`, not a direct import of the function name). This makes both the function and its `__Jig_<name>` marker struct visible to the macro-generated collection code. Same-module jigs can be referenced by name directly.
- The `jigs!(entry_fn)` macro must be placed in the same module as the entry function. It generates `all_jigs()` and `find_jig()`.
