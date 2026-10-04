# Flow engine: design and handoff

Flow is a general-purpose, file-backed controller for agent workflows.
The existing ten-step agent workflow is intended to be its first consumer,
not a hardcoded part of the engine. This document preserves the decisions
from NixOS task 0034 after the Rust crate moved into this workspace.
The separate NixOS task now covers putting the executable on PATH and
integrating it with Pi.

## Agreed model

- `Machine` is a reusable definition with an `initial_state` and states.
  Each `State` lists zero or more normal `next` destinations.
  `Next(target)` chooses one of those destinations; `JumpTo(target)` is an
  explicit exceptional move to any defined state. `Instance::apply_move`
  changes an instance in memory; `Instance::apply_saved_move` reloads and
  persists a named instance. Movement history is not recorded.
- An `Instance` has one current state and owns its loaded `Machine` in
  memory. The caller proposes moves for the engine to validate and apply.
  It does not execute workflow steps, prompt the user, run commands, or
  advance automatically. `Machine::into_new_instance(name)` consumes a
  machine and creates an in-memory instance at its initial state.
- Machine construction/loading rejects duplicate state names, an unknown
  initial state, and unknown normal-next targets. Instance construction
  validates its machine; loading checks that the current state exists in the
  loaded machine. Instance and machine fields are not publicly mutable, so
  normal API use preserves these invariants between moves. Direct Serde
  deserialization can still bypass constructor validation.
- Persistent per-instance **context** is planned for facts callers need
  across steps. It informs callers but does not guard moves. Its shape and
  update interface remain undecided; expected context keys in definitions
  are tentative.

## Files and lookup

- Local files are in `.flow/{machines,instances}/`; global files are in
  `$XDG_STATE_HOME/flow/{machines,instances}/`, defaulting to
  `~/.local/state/flow/`. Lookup is local-first and falls back globally only
  if the local path is absent, even for a globally stored instance.
  Consequently a local same-named machine can shadow a global definition.
- Machine TOML contains `initial_state` and `[[states]]` entries with `name`
  and `next`. Instance TOML contains only `machine` and `state`. Each file
  gets its own name from its filename, not from a field inside the file.
- `flow new <machine-name> [instance-name]` accepts an optional instance name.
  When omitted, it generates `<machine-name>-<32-hex-digit-UUID>`.
  `flow status` lists stored instances with their machine and state;
  `flow status [instance-name]` reports the known details for one instance.
  reports the known details for one instance. Local instances shadow same-named
  global instances in the listing, matching lookup behavior.
  `flow next <instance-name> <state>` follows a listed transition;
  `flow jump <instance-name> <state>` moves to any defined state.
  Explicit names cannot collide with instances visible in either scope.
  Generated names are machine-prefixed random UUIDs. UUID collisions are
  negligibly likely; persistence still uses no-overwrite creation.
- `new -g` stores the instance globally and requires a valid global machine
  definition. `new -G` implies global storage and copies a valid local
  machine definition if the global one is absent. It never replaces an
  existing global definition. Instance-name collisions are checked before
  copying; if instance creation later fails, an installed machine is left
  in place rather than risking deletion of a definition another process
  might use. The instance still loads its machine local-first.
- `Instance::save_new(InstanceSavePolicy)` resolves the destination, applies
  the global-machine requirement or copy option, then writes through a
  temporary file with no-overwrite persistence. Saved moves reload the
  instance and atomically replace that same path after validation, without
  recording history. Simultaneous writers are not coordinated and can lose
  moves. The temporary file is synced, but a power loss could still lose the
  rename. Files are short-lived CLI storage, not a resident service.
  `Path::exists` treats dangling symlinks as absent for lookup/prechecks;
  creation cannot overwrite one. Replacement replaces the path entry, even
  if it is a symlink, not the symlink's target.

## Current state and next engine work

The workspace contains the `flow` crate with `new` and `status`, validated
machine definitions, in-memory and durable movement, CLI tests, and a
panic-catching `main`.
The temporary demonstration panic was removed when the crate moved here.
The move and package rename passed formatting, Clippy, 49 tests, build,
`--help`, and a `new`/`status` smoke test. A later test audit reduced overlap;
40 tests now pass with durable movement. There is no movement history,
additional per-instance status reporting, context storage, or Pi extension yet.
Generated instance names use machine-prefixed random UUIDs.
The old `.agent-sm` storage paths were renamed to `.flow`; other projects'
old local files were not migrated automatically.

The first requested style pass introduced distinct validated `MachineName`
and `InstanceName` types. Machine and instance names use ASCII letters, digits,
`-`, `_`, and `.`, with an alphanumeric first character. CLI parsing and public
load/creation boundaries validate these names. State names remain
human-readable, but machine validation rejects empty/whitespace-only and
duplicate state names. Moving CLI command handlers onto command types remains
open. A second general style change was mentioned but not specified. Return to
Design with Zach before implementing each new slice; test and review before
moving to another. Defining additional per-instance status details is future
work. History and concurrent-write behavior remain open design questions,
along with hold/cancel/resume, informational guidance, definition
versioning and remaining malformed-definition checks. Add a second small
definition to verify that the engine remains generic.

## Environment

From `/home/zach/projects/agent-tools`:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p flow -- --help
```

The code is on branch `task/0034-scriptify-the-workflow` in this repository.
See the workspace [README](../README.md) for a minimal `new`/`status` example.
