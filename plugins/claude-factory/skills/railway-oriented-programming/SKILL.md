---
name: Railway-Oriented Programming
description: This skill should be used when the user or an agent is designing error handling, writing functions that can fail, chaining operations that may fail, or reviewing code for error-handling correctness. It explains Scott Wlaschin's railway-oriented programming approach — using Result/Either types to make the error path explicit and composable without exceptions or defensive coding.
version: 1.0.0
---

Railway-oriented programming (ROP) treats a fallible computation as a track with two rails: the happy path (success) and the error path. Functions that can fail take a value on the success rail and either continue on the success rail or switch to the error rail. Once on the error rail, subsequent functions are bypassed.

## The rules

- **All fallible operations return `Result<T, E>` (or `Either`/`Option` as appropriate)** — never throw exceptions for control flow, never return sentinel values (`-1`, `null`, `""`) to signal failure.
- **Error types are semantic types** — a function that can fail in three distinct ways has a specific error type with three variants, not a `String` or a generic `Error`.
- **No `unwrap()`, `expect()`, `panic!()` in production code** — these are the equivalent of throwing an untyped exception. Tests may use them when the intent is "this must not fail."
- **Chain with map/and_then/bind** — compose fallible operations without nested if/match. The railway metaphor: each function is a switch on the track.

## What ROP is not

ROP does not mean "catch all errors and convert to Result." Truly unrecoverable situations (out of memory, programmer errors caught by the type system) can still terminate. ROP is for expected failures: user input invalid, record not found, permission denied, network timeout.

## Per-language notes

- **Rust**: `Result<T, E>` + `?` operator. Use `thiserror` for error types, `anyhow` only at the application layer (never in library/domain code).
- **TypeScript**: `Result<T, E>` from `neverthrow` or hand-rolled. Do not use try/catch for control flow.
- **Python**: `Result` from `returns` library or hand-rolled dataclass union. Reserve exceptions for truly exceptional situations.
- **Haskell/F#**: native `Either`/`Result` with do-notation/computation expressions.

See `references/` for per-language chaining patterns.
