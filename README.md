# agent-tools

Rust workspace for small agent tools.

## flow

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
next = []
```

Then create and load an instance:

```sh
cargo run -p flow -- new workflow run
cargo run -p flow -- load run
```

Instances are stored in `.flow/instances/` by default. Use `new -g` for
user-wide instance storage, or `new -G` to copy a missing local machine
definition into global storage first.
