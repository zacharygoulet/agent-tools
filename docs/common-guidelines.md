# Common Guidelines

## Initial
Tool that helps AI agents follow configured deterministic flows. It does not do any
work itself, it just helps agents keep track of where they are in a complex multi-step
flow while driving the work ahead themselves.

Every flow define states, one is the initial state. All state define the possible next
states (none for final states). Each state document what it's about, and what the agent
should do when in that state.

You usually change state using the `next` command, or you can jump to any state using
the `jump` command. Required steps should always be done, contextual steps can depend
on the context and the AI's judgement.

## Every status

Required steps:
- Drive the flow proactively toward completion. Either progress, or give your best
guess of what's next, and provide the user with questions or simple confirmation.
- Document what you are doing in converstation. Write down your intents (before most actions), findings,
decisions and outcomes

Contextual steps:
- Load related skills
- Consider context compaction (historic context is mostly irrelevant, or bloated for upcomming work)
- Consider using a subagent (possible reasons: prevent context bloat, use specialized model, parallel work)
