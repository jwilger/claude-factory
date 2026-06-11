---
name: Event Modeling Method
description: This skill should be used when the user or an agent is creating or reviewing an event model, understanding event modeling methodology, decomposing a workflow into slices, or learning how to use emc for event modeling. It explains the event modeling methodology as practiced in Claude-Factory — Given/When/Then scenarios, slice decomposition, transition types, and workflow composition.
version: 1.0.0
---

Event modeling is a technique for planning systems by describing all the things that can happen (events), what causes them (commands, automations), and what state views they produce (read models). Claude-Factory uses emc to make these models formally verifiable.

## The four slice kinds

Every piece of behavior in the system is one of four kinds of slice:

- **state_change**: A command arrives → business rules are checked → events are emitted (or errors returned). This is where business logic lives. One command → one set of possible outcomes.
- **state_view**: Events are projected into a read model that can be queried. This is where "what does the UI display?" is answered. No commands; no mutations; only reads.
- **translation**: An external payload (HTTP body, message from another system) is translated into internal domain events. The bridge between your domain and the outside world.
- **automation**: A trigger (a domain event, a schedule, an external signal) automatically issues a command. No user interaction; purely reactive.

## Given / When / Then

Each slice has scenarios that describe its behavior:
- **Given**: the state of the world (relevant events that have already occurred)
- **When**: the trigger (the command for state_change, the projection event for state_view, etc.)
- **Then**: the result (new events emitted, or the projected read model, or the outcome/error)

A state_change slice must have scenarios for the happy path and all primary error cases.

## What is shared between slices

Only **events** are shared. Commands, views, read models, UI components, and business rules are owned by exactly one slice. This is the vertical slice boundary. Two slices that both react to the same event do so by each having their own projection logic — they do not share code.

## Workflow composition

Slices are organized into workflows. A workflow has:
- Exactly one entry step (the first slice a user or system reaches)
- Transitions between slices (via commands, events, navigation, external triggers, or outcomes)
- Every externally-relevant outcome connected to a continuation (another workflow or an explicit terminal state)

## emc's role

emc validates the structural rules (unique names, no dangling outcomes, etc.) and runs formal proofs (Lean4) and behavioral verification (Quint) against the model. A `WorkflowReadinessDeclared` event from emc is the gate for proceeding to development — it is not advisory.

See `references/` for detailed scenario-writing patterns and emc tool usage.
