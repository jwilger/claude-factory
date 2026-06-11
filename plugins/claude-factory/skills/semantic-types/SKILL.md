---
name: Semantic Types
description: This skill should be used when the user or an agent is defining domain types, writing function signatures, creating data structures, or reviewing code for type safety in a Claude-Factory managed project. It explains the "semantic types everywhere" constraint — the rule that every value in the system must carry its meaning in its type, not just its structure, including values used only within a single function body.
version: 1.0.0
---

Every value in the system must carry its meaning in its type. A `String` that represents a user ID and a `String` that represents an email address must be different types — the compiler must make it impossible to pass one where the other is expected.

## The constraint

- **No raw primitives for domain values** — not in function signatures, not in structs, not in local variables within a function body. Every integer, string, boolean, or collection that has domain meaning gets a semantic type.
- **I/O boundaries are the only exception** — when serializing/deserializing, JSON/database/wire formats use primitive types. The semantic type is constructed immediately at the boundary (parse, don't validate).
- **Serde-style serialization is fine** — code that teaches a type how to serialize/deserialize itself is not a violation. The type still exists; you're just giving it I/O capabilities.

## Parse, don't validate

Validation says: "take a String, check it, return a boolean." Parsing says: "take a String, check it, return a UserId or an error." After parsing, the type is the proof — you never re-validate.

```rust
// Wrong: validate then use the raw string
fn process(email: String) {
    assert!(is_valid_email(&email));  // validated but still just a String
    // ...
}

// Right: parse at the boundary, use the semantic type everywhere after
let email: EmailAddress = EmailAddress::parse(raw_string)?;
process(email);
```

## Why even function-local variables?

Two values that mean different things being the same type can be mixed up within a function body. This produces bugs that are impossible to detect at compile time. The cost of a newtype for a local variable is one line; the cost of a type confusion bug is a production incident.

See `references/` for per-language patterns (Rust nutype, TypeScript branded types, Python NewType, etc.).
