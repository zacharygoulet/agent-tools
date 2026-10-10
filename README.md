# flows

A file-backed CLI for creating and running configured workflows.

See the [design and handoff](docs/flow-design.md) for implementation details
and follow-up work.

Build and test from the repository root:

```sh
cargo build
cargo test
cargo run -- help
cargo run -- help autonomy set
```

Flow files live directly in `.flows/` or `$XDG_STATE_HOME/flows/`
(default `~/.local/state/flows/`). Create one from the bundled template, then
edit `.flows/workflow.toml` as needed:

```sh
cargo run -- new-flow-from-template workflow
cargo run -- list
cargo run -- list states workflow
```

`global.toml` is bundled with the executable and supplies optional `details`
and `steps` to instance status. A flow can set `use_global = false` to hide
that guidance. Flow and state `summary` fields appear in listings and status.

Then create, move, and inspect an instance:

```sh
cargo run -- start workflow run
cargo run -- next run Review
cargo run -- next run Done
cargo run -- status
cargo run -- status run
cargo run -- context set run goal "Finish review"
cargo run -- context remove run goal
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
