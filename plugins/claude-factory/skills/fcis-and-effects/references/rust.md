# FCIS and Effects — Rust

## Trait injection (the primary pattern)

Define the I/O contract as a trait in the functional core. The imperative shell provides the real implementation; tests provide a fake.

```rust
// In cfk-core (functional core) — trait defines the contract, no I/O
pub trait OrderRepository {
    fn find(&self, id: &OrderId) -> Result<Option<Order>, RepositoryError>;
    fn save(&self, order: &Order) -> Result<(), RepositoryError>;
}

// Pure function in the core — takes the trait, performs no I/O itself
pub fn place_order(
    repo: &impl OrderRepository,
    cmd: PlaceOrderCommand,
) -> Result<OrderPlaced, OrderError> {
    let existing = repo.find(&cmd.order_id)?;
    if existing.is_some() {
        return Err(OrderError::AlreadyExists(cmd.order_id));
    }
    let order = Order::new(cmd.order_id, cmd.items)?;
    repo.save(&order)?;
    Ok(OrderPlaced { order_id: order.id })
}
```

```rust
// In cfk-engine (imperative shell) — real implementation
pub struct PostgresOrderRepository { pool: sqlx::PgPool }

impl OrderRepository for PostgresOrderRepository {
    fn find(&self, id: &OrderId) -> Result<Option<Order>, RepositoryError> {
        // real SQL query
    }
    fn save(&self, order: &Order) -> Result<(), RepositoryError> {
        // real SQL insert/upsert
    }
}

// In tests — fake implementation (no mocking library)
pub struct InMemoryOrderRepository {
    orders: std::cell::RefCell<std::collections::HashMap<OrderId, Order>>,
}

impl OrderRepository for InMemoryOrderRepository {
    fn find(&self, id: &OrderId) -> Result<Option<Order>, RepositoryError> {
        Ok(self.orders.borrow().get(id).cloned())
    }
    fn save(&self, order: &Order) -> Result<(), RepositoryError> {
        self.orders.borrow_mut().insert(order.id.clone(), order.clone());
        Ok(())
    }
}
```

## Step/trampoline pattern for sequenced effects

When the core needs to request a sequence of I/O operations that depend on each other:

```rust
// The effect description — what I/O does the core need?
pub enum Effect {
    LoadUser { id: UserId },
    SaveEvent { event: DomainEvent },
    SendEmail { to: EmailAddress, template: EmailTemplate },
}

// The step type — either done or needs more I/O
pub enum Step<T> {
    Done(T),
    Effect(Effect, Box<dyn FnOnce(EffectResult) -> Step<T>>),
}

// Pure core function — returns a Step, performs no I/O
pub fn onboard_user(cmd: OnboardUserCommand) -> Step<Result<UserId, OnboardError>> {
    Step::Effect(
        Effect::LoadUser { id: cmd.id.clone() },
        Box::new(move |result| {
            let EffectResult::User(existing) = result else { unreachable!() };
            if existing.is_some() {
                return Step::Done(Err(OnboardError::AlreadyExists(cmd.id)));
            }
            let user = User::new(cmd.id.clone(), cmd.email.clone());
            Step::Effect(
                Effect::SaveEvent { event: DomainEvent::UserOnboarded { user } },
                Box::new(move |_| Step::Effect(
                    Effect::SendEmail { to: cmd.email, template: EmailTemplate::Welcome },
                    Box::new(|_| Step::Done(Ok(cmd.id))),
                )),
            )
        }),
    )
}

// Imperative shell — the trampoline
pub fn run_onboarding(cmd: OnboardUserCommand, shell: &impl Shell) -> Result<UserId, OnboardError> {
    let mut step = onboard_user(cmd);
    loop {
        match step {
            Step::Done(result) => return result,
            Step::Effect(effect, continuation) => {
                let result = shell.execute(effect);
                step = continuation(result);
            }
        }
    }
}
```

The trampoline pattern is most useful when the effect sequence is not known statically — when later effects depend on the results of earlier ones.

## The no-I/O-in-core rule for cfk

The kernel follows this strictly:
- `cfk-core` has zero I/O: no filesystem, no network, no randomness, no current time
- `cfk-engine` contains all I/O: event store, filesystem writes, forge API calls
- The core depends on no I/O crate — `std::io`, `tokio`, `sqlx`, `reqwest` are all in engine only
- Time injection: `chrono::Utc::now()` is called in the engine and passed as a parameter to core functions that need the current timestamp
