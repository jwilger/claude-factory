# Semantic Types — Rust

## nutype — the factory's primary tool

`nutype` generates newtype wrappers with validation, sanitization, and trait derives in one macro invocation.

```toml
# Cargo.toml
nutype = { version = "0.5", features = ["serde"] }
```

```rust
use nutype::nutype;

// String newtypes with validation
#[nutype(
    sanitize(trim, lowercase),
    validate(not_empty, len_char_max = 254, regex = r"^[^@]+@[^@]+\.[^@]+$"),
    derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)
)]
pub struct EmailAddress(String);

// Integer newtypes with range validation
#[nutype(
    validate(greater = 0, less_or_equal = 10_000),
    derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)
)]
pub struct Quantity(u32);

// UUID wrappers (semantic distinction: UserId ≠ ProductId)
#[nutype(derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize))]
pub struct UserId(uuid::Uuid);

impl UserId {
    pub fn new() -> Self {
        Self::try_new(uuid::Uuid::new_v4()).expect("uuid never fails validation")
    }
}
```

## Plain newtypes (without nutype)

For types that don't need validation — just semantic distinction:

```rust
// Wrapper struct — zero-cost in release builds
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SliceId(String);

impl SliceId {
    // Constructor is the only parse boundary — callers get SliceId, not String
    pub fn from_slug(slug: &str) -> Result<Self, InvalidSlug> {
        if slug.is_empty() || slug.contains(' ') {
            return Err(InvalidSlug(slug.to_string()));
        }
        Ok(Self(slug.to_string()))
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

## The boundary rule in practice

```rust
// Wrong — raw string crosses the boundary
fn send_email(to: String, subject: String, body: String) { ... }

// Right — all three are different types; compiler prevents transposition
fn send_email(to: EmailAddress, subject: EmailSubject, body: EmailBody) { ... }
```

## Local variables too

The "semantic types everywhere" rule applies inside function bodies, not just at API boundaries:

```rust
// Wrong — two ids both as String in the same scope
let user_id = record.user_id.clone();    // String
let product_id = record.product_id.clone(); // String — mixing is now a runtime bug

// Right
let user_id: UserId = record.user_id.clone();
let product_id: ProductId = record.product_id.clone();
// now the compiler catches transpositions
```

## serde boundaries

At the I/O boundary, teach the type to serialize/deserialize:

```rust
#[nutype(derive(Serialize, Deserialize))]  // nutype handles this
pub struct WorkItemId(uuid::Uuid);

// Or for plain newtypes:
impl<'de> Deserialize<'de> for WorkItemId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = uuid::Uuid::deserialize(d)?;
        Ok(WorkItemId(raw)) // no validation needed here; UUID is always valid
    }
}
```

The serde impl is the parse boundary — the semantic type is constructed here and used everywhere else.
