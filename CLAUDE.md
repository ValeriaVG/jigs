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
