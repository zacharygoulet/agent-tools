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

todo explain context

todo explain steps (global, flow, and state steps)


## Start / Resume steps
- establish autonomy

## Global details

todo explain autonomy options (autonomous, steered, guided), where:
- autonomous means the agent should run fully solo, user input should have been gathered
before starting, or the agent should use its best guess, stopping only on fundamental
issues

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
- have autonomy built in
- implement help command
- cli adjustements
- impl types display for prints instead of from cli
- review commands display (human readable)
- review globals.toml
- review error management (proper usage of panic/raise/result)
- trim tests
- update template.toml to show all features
