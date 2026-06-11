---
name: TDD Protocol
description: This skill should be used when the user or an agent is writing tests, reviewing tests, writing implementation code, or reviewing implementation code in a Claude-Factory managed project. It describes the enforced red-green-refactor discipline, the narrowest-change implementation rule, the recursive drill-down protocol for multi-function failures, and why behavioral tests must not couple to implementation.
version: 1.0.0
---

Claude-Factory enforces a strict TDD protocol with independent review gates at each step. The kernel enforces the sequence — agents must not skip ahead or anticipate future failures.

## The enforced sequence

1. **Write the outermost behavioral test** — black-box, exercises the slice from its public boundary
2. **Test review gate** — independent adversarial review (test-reviewer agent, potentially codex/GPT); veto loops until approved
3. **Kernel verifies red** — the kernel runs the test and confirms it fails for the expected reason (not any reason)
4. **Implement step** — address ONLY the first visible error; write no anticipatory code
5. **Kernel runs tests** — if a new first error appears, loop to step 4; if the failure requires >1 function change, drill down
6. **Drill-down protocol** — when a failure points at a multi-function gap, push a child TDD frame (tighter unit test for the missing piece); complete the child TDD cycle; pop the frame and resume the parent
7. **Green** — all tests pass; proceed to refactor
8. **Implementation review gate** — independent adversarial review; veto loops until approved
9. **Lint/format gate** — kernel runs linter at strictest project config; fix any violations
10. **Commit**

## Why this order matters

The outer test defines the contract. Implementation code written before the test exists cannot be verified to satisfy the contract. Code written in anticipation of future failures adds untested paths that may contain bugs or violate architectural constraints.

## The narrowest-change rule

At step 4, "narrowest change" means: change the fewest lines of code in the fewest places to resolve exactly the current first error. If resolving that error also requires changing a second unrelated function, that is a signal the first error is pointing at a missing abstraction — drill down, do not push through.

## Behavioral vs implementation coupling

A test is behavioral if: replacing the entire implementation with a different algorithm that produces the same observable outputs would not break the test. A test is implementation-coupled if it: asserts internal state, calls internal methods, or relies on the specific call sequence of dependencies.

See `references/` for per-language examples.
