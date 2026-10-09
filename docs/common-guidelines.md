# Common Guidelines

## Help
Tool that helps AI agents follow configured deterministic flows. It does not do any
work itself, it just helps agents keep track of where they are in a complex multi-step
flow while driving the work ahead themselves.

Every flow define states, one is the initial state. All state define the possible next
states (none for final states). Each state document what it's about, and what the agent
should do when in that state.

You usually change state using the `next` command, or you can jump to any state using
the `jump` command. Required steps should always be done, contextual steps can depend
on the context and the AI's judgement.

### Autonomy

Each instance starts with **guided** autonomy for every state. The current state's
level appears in `flow status <instance-name>` and determines how the agent should
handle decisions while working in that state:

- **Guided:** Pause for user approval at state boundaries and significant decisions.
  Still drive the work forward by proposing the next action or a clear question.
- **Steered:** Proceed by default, report progress, and ask the user about
  consequential choices rather than routine steps.
- **Autonomous:** Work independently. Gather needed user input before starting,
  or make the best reasonable guess. Stop only for fundamental blockers, not for
  routine approval. Continue to report intentions, findings, and outcomes.

These levels guide the agent; Flow does not perform or enforce the work itself.
To change one state's level, run
`flow autonomy set <instance> <guided|steered|autonomous> <state>`.
Add an end state to change every state on any `next` path from the first
state through the end state:
`flow autonomy set <instance> <level> <first-state> <last-state>`.
Quote state names containing spaces. Branches that cannot reach the end state
are excluded, and traversal stops at the end even if it has outgoing links.
If there is no `next` path between the endpoints, the command fails without
changing anything. The command lists every affected state in definition-file
order for readability; definition order does not determine which states are
included. Settings persist on the instance and stay with their states when it
moves. Use `jump` for exceptional moves outside the normal `next` flow.

todo explain context

todo explain steps (global, flow, and state steps)


## Start / Resume steps
- establish autonomy

## Global details

Follow the current state's autonomy level as described in Help.

## Global steps

- Drive the flow proactively toward completion. Either progress, or give your best
guess of what's next, and provide the user with questions or simple confirmation.
- Document what you are doing in converstation. Write down your intents (before most actions), findings,
decisions and outcomes
- Load related skills
- Consider context compaction (historic context is mostly irrelevant, or bloated for upcomming work)
- Consider using a subagent (possible reasons: prevent context bloat, use specialized model, parallel work)



Cli adjustents:

- Same first command to explain the tool
  - Explains the tool
  - Show status (available definitions and instances)
- Start (new instance)
- Resume (resume instance, similar to current start with a name)
- Move (Next or Jump)
- Pause (Global )


## Todos
- implement help command
- cli adjustements
- impl types display for prints instead of from cli
- review commands display (human readable)
- review globals.toml
- review error management (proper usage of panic/raise/result)
- trim tests
- update template.toml to show all features
