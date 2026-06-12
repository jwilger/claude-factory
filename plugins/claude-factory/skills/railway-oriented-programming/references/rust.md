# Railway-Oriented Programming — Rust

## The `?` operator — idiomatic ROP

Rust's `?` operator is the language-native ROP chain operator. It short-circuits on `Err`, propagating the error to the caller.

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrderError {
    #[error("product {0} not found")]
    ProductNotFound(ProductId),
    #[error("insufficient inventory: need {needed}, have {available}")]
    InsufficientInventory { needed: Quantity, available: Quantity },
    #[error("payment declined: {reason}")]
    PaymentDeclined { reason: String },
}

fn place_order(
    product_id: ProductId,
    quantity: Quantity,
    payment: PaymentMethod,
) -> Result<OrderId, OrderError> {
    let product = find_product(product_id)?;             // short-circuits on NotFound
    let _ = reserve_inventory(&product, quantity)?;       // short-circuits on Insufficient
    let order_id = charge_payment(payment, product.price * quantity)?; // short-circuits on Declined
    Ok(order_id)
}
```

## thiserror for domain error types

```toml
# Cargo.toml
thiserror = "2"
```

```rust
use thiserror::Error;

// One error type per domain operation — not a flat "AppError" that catches everything
#[derive(Debug, Error)]
pub enum RegistrationError {
    #[error("email already registered")]
    EmailTaken,
    #[error("invalid email format")]
    InvalidEmail(#[from] EmailValidationError),
    #[error("password too weak: {0}")]
    WeakPassword(String),
}

// Conversion between error types for cross-layer composition
#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("registration failed: {0}")]
    Registration(#[from] RegistrationError),
    #[error("authentication failed: {0}")]
    Auth(#[from] AuthError),
    // Add variants only for errors that originate HERE — don't catch-all
}
```

**`anyhow` rule:** use `anyhow::Error` only at the application/binary layer (main, CLI entry points). Never in library or domain code — it erases the error type and makes callers unable to match on specific failures.

## Map and and_then for transformation

```rust
fn validated_quantity(raw: i64) -> Result<Quantity, QuantityError> {
    raw.try_into()
        .map_err(|_| QuantityError::Negative)       // map: transform the error type
        .and_then(|q: u32| {                         // and_then: chain a second fallible operation
            if q == 0 { Err(QuantityError::Zero) }
            else { Ok(Quantity::try_new(q).expect("already checked > 0")) }
        })
}
```

## Multiple errors in parallel (collect into Result)

```rust
// Validate a batch — collect all errors, not just the first
fn validate_items(raw_items: Vec<RawItem>) -> Result<Vec<Item>, Vec<ValidationError>> {
    let results: Vec<Result<Item, ValidationError>> = raw_items
        .into_iter()
        .map(|raw| validate_item(raw))
        .collect();
    
    let errors: Vec<_> = results.iter().filter_map(|r| r.as_ref().err().cloned()).collect();
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(results.into_iter().map(|r| r.unwrap()).collect())
}
```

## Error type design principles

- One error enum per fallible domain operation (not per module, not one global AppError)
- Variants are meaningful to callers — `PaymentDeclined` not `PaymentError(String)`
- `#[from]` for automatic conversion chains between layers
- Never `Box<dyn Error>` in domain code — it hides the type from pattern matching
- Clippy enforces `clippy::result_large_err` — if your error type is large, box the payload not the enum
