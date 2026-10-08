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

Definitions live in `.flow/definitions/` or
`$XDG_STATE_HOME/flow/definitions/` (default `~/.local/state/flow/definitions/`).
Create a definition from the bundled three-state template, then edit
`.flow/definitions/workflow.toml` as needed:

```sh
cargo run -p flow -- new-definition-from-template workflow
```

Then create, move, and inspect an instance:

```sh
cargo run -p flow -- start workflow run
cargo run -p flow -- next run Review
cargo run -p flow -- next run Done
cargo run -p flow -- status
cargo run -p flow -- status run
cargo run -p flow -- context set run goal "Finish review"
cargo run -p flow -- context remove run goal
```

Context is a persistent string-to-string map in the instance's `[context]`
TOML table. `status run` shows its entries when present.

Use `jump` instead of `next` for an exceptional move to any defined state.
Instances are stored in `.flow/instances/` (with `.toml` files) by default. Use `start -g` for
user-wide instance storage, or `start -G` to copy a missing local definition
into global storage first. Add `-g` to
`new-definition-from-template` to create a global definition.
