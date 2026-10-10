# Flows engine: design and handoff

Flows is a general-purpose, file-backed controller for agent workflows. The
existing ten-step agent workflow is intended to be its first consumer, not a
hardcoded part of the engine. The separate NixOS task covers putting the
executable on PATH and integrating it with Pi.

## Model

- A `Flow` describes a reusable workflow with an `initial_state` and states.
  Each `State` lists zero or more normal `next` destinations. `Next(target)`
  chooses one of those destinations; `JumpTo(target)` is an explicit exceptional
  move to any defined state. `Instance::apply_move` changes an instance in
  memory; `Instance::load_from_name(name).move_to(movement)` resolves the
  instance path again before persisting a move. Movement history is not
  recorded.
- An `Instance` has one current state and owns its loaded `Flow` in memory. The
  caller proposes moves for the engine to validate and apply. It does not
  execute workflow steps, prompt the user, run commands, or advance
  automatically. `Flow::into_new_instance(name)` consumes a flow and creates an
  in-memory instance at its initial state.
- Flow construction/loading rejects duplicate state names, an unknown initial
  state, and unknown normal-next targets. Instance construction validates its
  flow; loading checks that the current state exists in the loaded flow.
  Instance and flow fields are not publicly mutable, so normal API use preserves
  these invariants between moves. Direct Serde deserialization can still bypass
  constructor validation.
- Persistent per-instance context is a string-to-string map for compact
  reminders and pointers callers need across states. The CLI limits values to
  150 characters. Context informs callers but does not guard moves. Instances
  without context load as an empty map.

## Files and lookup

- Local flow files live directly in `.flows/` (for example,
  `.flows/workflow.toml`); local instances live in `.flows/instances/`.
  Global flow files live directly in `$XDG_STATE_HOME/flows/`, and global
  instances in its `instances/` subdirectory. The default state directory is
  `~/.local/state/flows/`.
- Lookup is local-first and falls back globally only if the local path is
  absent, even for a globally stored instance. Consequently a local same-named
  flow can shadow a global flow.
- The bundled `flows/global.toml` supplies `details` and `steps` to every
  instance status, unless its flow has `use_global = false`. It is embedded in
  the executable rather than read from user storage; malformed guidance fails
  status.
- Flow TOML contains `initial_state` and `[[states]]` entries with `name` and
  `next`. Flows and states may also include `summary`, `details`, and `steps`.
  These are static guidance, not tracked checklists or transition guards.
  Instance TOML contains `flow`, `state`, and optional `[context]` and
  `[autonomy]` tables. State and instance names come from the TOML filename,
  not from a field inside the file.
- `flows start <flow-name> [instance-name]` creates an instance. When the
  instance name is omitted, it generates `<flow-name>-<32-hex-digit-UUID>`.
  `flows new-flow-from-template <flow-name> [-g]` creates a flow from
  `flows/templates/flow.toml`, locally by default or globally with `-g`. It
  does not overwrite an existing flow in the destination scope.
- `flows status` lists stored instances; `flows status [instance-name]`
  reports guidance and context for one instance. `flows list` lists flow names
  and summaries, and `flows list states <flow-name>` lists states and their
  summaries in flow order. Local files shadow same-named global files in
  listings, matching lookup behavior.
- `flows next <instance-name> <state>` follows a listed transition;
  `flows jump <instance-name> <state>` moves to any defined state.
  `flows context set <instance-name> <key> <value>` adds or replaces a context
  value; `flows context remove <instance-name> <key>` removes one.
  `flows autonomy set` configures an instance's autonomy for one state or a
  range of states reachable along listed next paths.
- `start -g` stores an instance globally and requires a valid global flow.
  `start -G` implies global storage and copies a valid local flow if the global
  one is absent. It never replaces an existing global flow. Instance-name
  collisions are checked before copying; if instance creation later fails, an
  installed flow is left in place rather than risking deletion of a flow
  another process might use. The instance still loads its flow local-first.
- `Instance::save_new(InstanceSavePolicy)` resolves the destination, applies
  the global-flow requirement or copy option, then writes through a temporary
  file with no-overwrite persistence. Saved moves reload the instance and
  resolve its path again before atomically replacing it after validation,
  without recording history. Context edits use the same replacement
  mechanism. Lookup is local-first at save time, so a newly appearing local
  instance can shadow one loaded globally. Simultaneous writers are not
  coordinated and can lose moves or context edits. The temporary file is
  synced, but a power loss could still lose the rename. Files are short-lived
  CLI storage, not a resident service.

## Current state and follow-up

The workspace contains the `flows` crate with `start`,
`new-flow-from-template`, and `status`; validated flows; in-memory and durable
movement; CLI tests; and a panic-catching `main`. There is no movement history
or Pi extension yet. Generated instance names use flow-prefixed random UUIDs.

The previous storage layout was `.flow/{definitions,instances}/` locally and
`$XDG_STATE_HOME/flow/{definitions,instances}/` globally; instance files used
the `definition` key. No automatic migration is performed. The new flow file
layout is `.flows/<flow-name>.toml`,
instances live in `.flows/instances/`, and the instance reference key is `flow`.
Existing tooling or stored files using the old executable name, paths, or
instance field will need to be updated.

Potential future work includes movement history, concurrent-write behavior,
hold/cancel/resume, flow versioning, remaining malformed-flow checks, and
verifying the engine with additional flows. Moving CLI command handlers onto
command types also remains open.

## Environment

From the workspace root:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p flows -- --help
```

See the workspace [README](../README.md) for flow-template and
`start`/`status` examples.
