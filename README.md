# agent-tools

Rust workspace for small agent tools.

## flows

See the [Flows design and handoff](docs/flow-design.md) for decisions and
remaining work.

Build and test from the workspace root:

```sh
cargo build -p flows
cargo test -p flows
cargo run -p flows -- help
cargo run -p flows -- help autonomy set
```

Flow files live directly in `.flows/` or `$XDG_STATE_HOME/flows/`
(default `~/.local/state/flows/`). Create one from the bundled template, then
edit `.flows/workflow.toml` as needed:

```sh
cargo run -p flows -- new-flow-from-template workflow
cargo run -p flows -- list
cargo run -p flows -- list states workflow
```

`flows/global.toml` is bundled with the executable and supplies optional
`details` and `steps` to instance status. A flow can set `use_global = false` to hide
that guidance. Flow and state `summary` fields appear in listings and status.

Then create, move, and inspect an instance:

```sh
cargo run -p flows -- start workflow run
cargo run -p flows -- next run Review
cargo run -p flows -- next run Done
cargo run -p flows -- status
cargo run -p flows -- status run
cargo run -p flows -- context set run goal "Finish review"
cargo run -p flows -- context remove run goal
```

An instance is stored in `.flows/instances/` (with `.toml` files) by default.
Its TOML uses the `flow` key to reference the flow file. Use `start -g` for
user-wide instance storage, or `start -G` to copy a missing local flow into
global storage first. Add `-g` to `new-flow-from-template` to create a global
flow.

Context is a persistent string-to-string map in the instance's `[context]`
TOML table. `status run` shows its entries when present. Keep values compact:
short reminders and pointers (such as an issue ID or plan URL), not detailed
plans, requirements, or notes. The CLI limits each value to 150 characters.

Use `jump` instead of `next` for an exceptional move to any defined state.
