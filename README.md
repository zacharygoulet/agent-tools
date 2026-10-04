# agent-tools

Rust workspace for small agent tools.

## flow

See the [engine design and handoff](docs/flow-design.md) for decisions and
remaining work.

Build and test from the workspace root:

```sh
cargo build -p flow
cargo test -p flow
cargo run -p flow -- --help
```

Machines live in `.flow/machines/` or
`$XDG_STATE_HOME/flow/machines/` (default `~/.local/state/flow/machines/`).
For example, create `.flow/machines/workflow`:

```toml
initial_state = "Design"

[[states]]
name = "Design"
next = ["Review"]

[[states]]
name = "Review"
next = []
```

Then create, move, and inspect an instance:

```sh
cargo run -p flow -- new workflow run
cargo run -p flow -- next run Review
cargo run -p flow -- status
cargo run -p flow -- status run
```

Use `jump` instead of `next` for an exceptional move to any defined state.
Instances are stored in `.flow/instances/` by default. Use `new -g` for
user-wide instance storage, or `new -G` to copy a missing local machine
definition into global storage first.
