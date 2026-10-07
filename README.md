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
Create a machine from the bundled three-state template, then edit
`.flow/machines/workflow.toml` as needed:

```sh
cargo run -p flow -- new-machine-from-template workflow
```

Then create, move, and inspect an instance:

```sh
cargo run -p flow -- start workflow run
cargo run -p flow -- next run Review
cargo run -p flow -- next run Done
cargo run -p flow -- status
cargo run -p flow -- status run
```

Use `jump` instead of `next` for an exceptional move to any defined state.
Instances are stored in `.flow/instances/` (with `.toml` files) by default. Use `start -g` for
user-wide instance storage, or `start -G` to copy a missing local machine
definition into global storage first. Add `-g` to
`new-machine-from-template` to create a global machine definition.
