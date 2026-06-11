---
name: Functional Core / Imperative Shell and Effects
description: This skill should be used when the user or an agent is designing functions that need I/O, deciding where to put business logic, or implementing the effects pattern for requesting I/O from the functional core. It explains functional-core/imperative-shell architecture and the effects (or step/trampoline) pattern for languages that do not natively support algebraic effects.
version: 1.0.0
---

All important business logic lives in pure functions. All I/O happens at the boundaries. The functional core cannot perform I/O — it can only request it.

## Functional-core / imperative-shell

- **Functional core**: pure functions that take data in and return data out. No database calls, no HTTP, no filesystem, no randomness, no current time. Completely deterministic. Trivially testable without any infrastructure.
- **Imperative shell**: the thin outer layer that calls real I/O (database, HTTP, files, etc.), feeds results into the core, and executes the core's output instructions.

The boundary is explicit: the shell calls the core; the core never calls the shell.

## The effects pattern

When the functional core needs to *request* an I/O operation (e.g., "load this user from the database before I can compute the result"), it returns an effect description instead of performing the I/O directly.

**Languages with native effect systems** (Haskell, Koka, Effekt): use the native mechanism.

**Languages without native effects** (Rust, TypeScript, Python): use one of:
- **Trait/interface injection**: the core accepts an interface parameter; the shell provides a real implementation; tests provide a fake one
- **Step/trampoline pattern**: the core returns a `Step` enum — either `Done(result)` or `Effect(description, continuation)`; the shell runs the trampoline loop

The step/trampoline pattern is particularly useful when the effects form a sequence that is hard to express as simple dependency injection. See `references/` for per-language examples.

## Why this matters for testability

A pure functional core can be tested with no test infrastructure — just call the function with inputs and assert outputs. I/O tests (shell tests) are integration tests that test only the wiring, not the logic. This separation is what makes the factory's "no mocking libraries" rule feasible: the core doesn't need mocks because it has no I/O to mock.
