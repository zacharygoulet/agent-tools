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
  validates and applies these moves in memory; persistence and history are
  not implemented.
- An `Instance` has one current state and owns its loaded `Machine` in
  memory. The caller will propose moves for the engine to validate and
  record. It does not execute workflow steps, prompt the user, run commands,
  or
  advance automatically. `Machine::into_new_instance(name)` consumes a
  machine and creates an in-memory instance at its initial state.
- Machine construction/loading rejects duplicate state names, an unknown
  initial state, and unknown normal-next targets. Loading an instance checks
  that its current state exists in the loaded machine. Public fields and
  direct Serde deserialization can bypass some constructor validation.
- Machine definitions are expected to remain unchanged while instances
  use them, but immutability is not enforced.
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
- `flow new <machine-name> [instance-name]` currently requires an explicit
  instance name. `flow load <instance-name>` uses no machine argument.
  Explicit names cannot collide with instances visible in either scope.
  Generated names should eventually be machine-prefixed unique IDs; their
  exact format is undecided.
- `new -g` stores the instance globally and requires a valid global machine
  definition. `new -G` implies global storage and copies a valid local
  machine definition if the global one is absent. It never replaces an
  existing global definition. Instance-name collisions are checked before
  copying; if instance creation later fails, an installed machine is left
  in place rather than risking deletion of a definition another process
  might use. The instance still loads its machine local-first.
- `Instance::save_new(InstanceSavePolicy)` resolves the destination, applies
  the global-machine requirement or copy option, then writes through a
  temporary file with no-overwrite persistence. Files are short-lived CLI
  storage, not a resident service. `Path::exists` treats dangling symlinks
  as absent for lookup/prechecks, but the final write cannot overwrite one.

## Current state and next engine work

The workspace contains the `flow` crate with `new` and `load`, validated
machine definitions, in-memory movement, persistence for new instances,
CLI tests, and a panic-catching `main`.
The temporary demonstration panic was removed when the crate moved here.
The move and package rename passed formatting, Clippy, 49 tests, build,
`--help`, and a `new`/`load` smoke test. A later test audit reduced overlap;
35 tests now pass with in-memory movement. There is no durable movement or
history, status reporting, context storage, Pi extension, or generated-name
implementation yet.
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
moving to another. The next movement slice needs a design for durable updates
and history, including failure and concurrent-write behavior. Other open
questions include hold/cancel/resume, informational guidance, definition
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
See the workspace [README](../README.md) for a minimal `new`/`load` example.
