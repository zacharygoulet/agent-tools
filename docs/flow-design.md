# Flow engine: design and handoff

Flow is a general-purpose, file-backed controller for agent workflows.
The existing ten-step agent workflow is intended to be its first consumer,
not a hardcoded part of the engine. This document preserves the decisions
from NixOS task 0034 after the Rust crate moved into this workspace.
The separate NixOS task now covers putting the executable on PATH and
integrating it with Pi.

## Agreed model

- `Definition` describes a reusable flow with an `initial_state` and states.
  Each `State` lists zero or more normal `next` destinations.
  `Next(target)` chooses one of those destinations; `JumpTo(target)` is an
  explicit exceptional move to any defined state. `Instance::apply_move`
  changes an instance in memory; `Instance::load_from_name(name).move_to(movement)`
  resolves the instance path again before persisting a move. Movement history
  is not recorded.
- An `Instance` has one current state and owns its loaded `Definition` in
  memory. The caller proposes moves for the engine to validate and apply.
  It does not execute workflow steps, prompt the user, run commands, or
  advance automatically. `Definition::into_new_instance(name)` consumes a
  flow definition and creates an in-memory instance at its initial state.
- Definition construction/loading rejects duplicate state names, an unknown
  initial state, and unknown normal-next targets. Instance construction
  validates its definition; loading checks that the current state exists in the
  loaded definition. Instance and definition fields are not publicly mutable, so
  normal API use preserves these invariants between moves. Direct Serde
  deserialization can still bypass constructor validation.
- Persistent per-instance **context** is a string-to-string map for compact
  reminders and pointers callers need across states (for example, an issue ID
  or plan URL). It is not a place for detailed plans, requirements, or notes.
  The CLI limits values to 150 characters. Context informs callers but does not
  guard moves. Instances without context load as an empty map.

## Files and lookup

- Local files are in `.flow/{definitions,instances}/` with `.toml` extensions;
  global files are in `$XDG_STATE_HOME/flow/{definitions,instances}/`, defaulting to
  `~/.local/state/flow/`. Lookup is local-first and falls back globally only
  if the local path is absent, even for a globally stored instance.
  Consequently a local same-named definition can shadow a global definition.
- The bundled `flow/global.toml` supplies `details` and `steps` to every
  instance status, unless its definition has `use_global = false`. It is
  embedded in the executable rather than read from global instance storage;
  malformed guidance fails status. The first draft draws from
  `docs/common-guidelines.md`; that document remains unchanged.
- Definition TOML contains `initial_state` and `[[states]]` entries with `name`
  and `next`. Definitions and states may also include `summary`, `details`,
  and `steps`. These are static guidance, not tracked checklists or transition
  guards. Step optionality can be expressed in the step or its details.
  `flow status <instance-name>` groups global, definition, and current-state
  guidance by scope. Definition and state summaries also appear in status;
  immediate next states show their summaries. Definition summaries apply
  to listings, while state summaries describe states in a definition.
  Instance TOML contains `definition`, `state`, and an optional `[context]`
  table of string values. Each file gets its name from its filename without
  the `.toml` extension, not from a field inside the file.
- `flow start <definition-name> [instance-name]` creates an instance and accepts
  an optional instance name. When omitted, it generates
  `<definition-name>-<32-hex-digit-UUID>`.
  `flow new-definition-from-template <definition-name> [-g]` creates a definition
  from `flow/templates/definition.toml`, locally by default or
  globally with `-g`. It does not overwrite an existing definition in the
  destination scope.
  `flow status` lists stored instances with their definition and state;
  `flow status [instance-name]` reports guidance and context for one instance.
  `flow list definitions` lists definition names and summaries, and
  `flow list states <definition-name>` lists states and their summaries in
  definition order. Local files shadow same-named global files in listings,
  matching lookup behavior.
  `flow next <instance-name> <state>` follows a listed transition;
  `flow jump <instance-name> <state>` moves to any defined state.
  `flow context set <instance-name> <key> <value>` adds or replaces a value
  of at most 150 characters; values should be compact reminders or pointers.
  `flow context remove <instance-name> <key>` removes an existing value.
  `flow status <instance-name>` displays nonempty context as TOML entries.
  Explicit names cannot collide with instances visible in either scope.
  Generated names are definition-prefixed random UUIDs. UUID collisions are
  negligibly likely; persistence still uses no-overwrite creation.
- `start -g` stores the instance globally and requires a valid global
  definition. `start -G` implies global storage and copies a valid local
  definition if the global one is absent. It never replaces an
  existing global definition. Instance-name collisions are checked before
  copying; if instance creation later fails, an installed definition is left
  in place rather than risking deletion of a definition another process
  might use. The instance still loads its definition local-first.
- `Instance::save_new(InstanceSavePolicy)` resolves the destination, applies
  the global-definition requirement or copy option, then writes through a
  temporary file with no-overwrite persistence. Saved moves reload the
  instance and resolve its path again before atomically replacing it after
  validation, without recording history.
  `Instance::load_from_name(name).update_context(update)` uses the same
  replacement mechanism for context edits. Lookup is local-first at save time,
  so a newly appearing local instance can shadow one loaded globally.
  Simultaneous writers are not coordinated and can lose moves or context
  edits. The temporary file is synced, but a power loss could still lose the
  rename. Files are short-lived CLI storage, not a resident service.
  `Path::exists` treats dangling symlinks as absent for lookup/prechecks;
  creation cannot overwrite one. Replacement replaces the path entry, even
  if it is a symlink, not the symlink's target.

## Current state and next engine work

The workspace contains the `flow` crate with `start`,
`new-definition-from-template`, and `status`; validated definitions;
in-memory and durable movement; CLI tests; and a panic-catching `main`.
The temporary demonstration panic was removed when the crate moved here.
Verification includes formatting, Clippy, CLI tests, and definition-template
and start/status smoke tests. There is no movement history or Pi extension yet.
Generated instance names use definition-prefixed random UUIDs.
The old `.agent-sm` storage paths were renamed to `.flow`; other projects'
old local files were not migrated automatically. The later `machines` to
`definitions` rename is a clean break: old paths, the instance `machine` key,
and old CLI names are not supported.

The first requested style pass introduced distinct validated `DefinitionName`
and `InstanceName` types. Definition and instance names use ASCII letters, digits,
`-`, `_`, and `.`, with an alphanumeric first character. CLI parsing and public
load/creation boundaries validate these names. State names remain
human-readable, but definition validation rejects empty/whitespace-only and
duplicate state names. Moving CLI command handlers onto command types remains
open. A second general style change was mentioned but not specified. Return to
Design with Zach before implementing each new slice; test and review before
moving to another. History and concurrent-write behavior remain open design
questions, along with hold/cancel/resume, definition versioning, and remaining
malformed-definition checks. Add a second small
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
See the workspace [README](../README.md) for definition-template and
`start`/`status` examples.
