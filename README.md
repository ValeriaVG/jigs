# jigs

> ⚠️ **Very early stage.** This project is in active, exploratory development and is nowhere close to being production ready. APIs will change, things will break, and entire ideas may be thrown out. Star it, watch it, play with it — but don't ship it.

A jig is a custom-made tool that controls the location or motion of other tools.

HTTP handlers, RPC services, agentic workflows, data pipelines and even event listeners all have the same shape: something comes in and, after several steps, something comes out.

And the more steps there are in this process, the harder it becomes to debug, fix and maintain the code that does it.

So `jigs` is a small Rust framework that lets you be explicit about your processing steps and write your code like this:

```rust
#[jig]
let pipeline = request
    .then(log_incoming)
    .then(set_auth_state)
    .then(route_to_feature)
    .then(log_response);
```

Every step is a `jig` too, so you can compose processing pipelines of virtually any size.

The main advantage, though, is that `jigs` generates a graph of your codebase at compile time and lets you navigate it with ease.

## Trace the exact steps in runtime

Things break, and the bigger the system, the harder it is to figure out where exactly things went wrong. `jigs` lets you pinpoint an error or a bottleneck with ease:
```
log_incoming     ✓  0.1ms
set_auth_state   ✓  2.3ms
route_to_feature ✗  ERROR 500
  └─ validate_route  ✓  0.4ms
  └─ check_permissions ✗  ERROR: DB Timeout
```

## Everything agnostic

`jigs` works with any existing or future framework that has two distinct types for incoming and outbound messages. Check the [examples](./examples) to see it in action.

## Roadmap
- [ ] Basic functionality
- [ ] Time tracing and logging utils
- [ ] Generation of interactive map at compile time

## Maybe Roadmap
- Ability to trace and view remote jigs services as one system
- Self hosting and cloud options


## License

MIT — see [LICENSE](./LICENSE).
