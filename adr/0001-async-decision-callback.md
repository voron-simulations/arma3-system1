# 0001: Asynchronous decision via extension callback

## Status
Accepted

## Context
The model is queried over HTTP. `callExtension` blocks the game thread, and a model call can take seconds.

## Decision
`decide` renders the context, spawns a worker thread and returns `OK` immediately. The worker reports with
`callback_data("system1", "decision" | "error", ...)`, handled by the `ExtensionCallback` event handler.
Panics in the worker are caught and reported as `error`, otherwise the group's in-flight flag would never clear.

SQF dispatches callbacks through a whitelist (`decision`, `error`) rather than by name from `missionNamespace`.

## Consequences
- Requests are tracked per group (`system1_decision_inFlight` group variable), so one slow group doesn't block others.
- Group ids are round-tripped through the extension, since a callback can't carry a group reference.
- A decision may arrive after its group died; it is dropped.
