# FCIS and Effects — TypeScript

## Dependency injection via interfaces

```typescript
// Port — the I/O contract, defined in the domain layer
interface OrderRepository {
    find(id: OrderId): Promise<Result<Order | null, RepositoryError>>;
    save(order: Order): Promise<Result<void, RepositoryError>>;
}

interface EmailService {
    send(to: EmailAddress, template: EmailTemplate): Promise<Result<void, EmailError>>;
}

// Pure-ish domain function — depends on interfaces, not implementations
async function onboardUser(
    repo: OrderRepository,
    email: EmailService,
    cmd: OnboardUserCommand,
): Promise<Result<UserId, OnboardError>> {
    const existing = await repo.find(cmd.userId);
    if (existing.isErr()) return err(toOnboardError(existing.error));
    if (existing.value !== null) return err({ kind: 'already_exists', userId: cmd.userId });

    const user = createUser(cmd);
    const saved = await repo.save(user);
    if (saved.isErr()) return err(toOnboardError(saved.error));

    await email.send(cmd.email, EmailTemplate.Welcome); // best-effort; ignore failure
    return ok(user.id);
}
```

```typescript
// Adapter — real implementation (in the infrastructure layer)
class PostgresOrderRepository implements OrderRepository {
    constructor(private readonly pool: Pool) {}

    async find(id: OrderId): Promise<Result<Order | null, RepositoryError>> {
        const row = await this.pool.query('SELECT * FROM orders WHERE id = $1', [id]);
        return ok(row.rows[0] ? mapRowToOrder(row.rows[0]) : null);
    }
    // ...
}

// Fake — for tests, no library needed
class InMemoryOrderRepository implements OrderRepository {
    private readonly store = new Map<OrderId, Order>();

    async find(id: OrderId): Promise<Result<Order | null, RepositoryError>> {
        return ok(this.store.get(id) ?? null);
    }

    async save(order: Order): Promise<Result<void, RepositoryError>> {
        this.store.set(order.id, order);
        return ok(undefined);
    }
}
```

## Separating pure logic from I/O

When a function has pure logic and I/O mixed, extract the pure part:

```typescript
// Wrong — computation and I/O tangled
async function calculateDiscount(userId: UserId, cartId: CartId): Promise<Money> {
    const user = await userRepo.find(userId);       // I/O
    const cart = await cartRepo.find(cartId);       // I/O
    const tier = user.subscriptionTier;
    const subtotal = cart.items.reduce((sum, item) => sum + item.price, 0 as Money);
    return subtotal * discountRate(tier);           // pure
}

// Right — pure logic is separately testable
function applyDiscount(tier: SubscriptionTier, subtotal: Money): Money {
    return (subtotal * discountRate(tier)) as Money;
}

async function calculateDiscount(userId: UserId, cartId: CartId): Promise<Money> {
    const [user, cart] = await Promise.all([userRepo.find(userId), cartRepo.find(cartId)]);
    return applyDiscount(user.subscriptionTier, cart.subtotal);
}
```

## Effect types (advanced)

For complex sequences where later effects depend on earlier results, use an explicit `Effect` union:

```typescript
type Effect =
    | { readonly kind: 'load_user'; readonly id: UserId }
    | { readonly kind: 'save_event'; readonly event: DomainEvent }
    | { readonly kind: 'send_email'; readonly to: EmailAddress; readonly template: EmailTemplate };

type Step<T> =
    | { readonly kind: 'done'; readonly value: T }
    | { readonly kind: 'effect'; readonly effect: Effect; readonly continue: (result: EffectResult) => Step<T> };

// Pure — returns a Step description, performs no I/O
function onboardUserSteps(cmd: OnboardUserCommand): Step<Result<UserId, OnboardError>> {
    return {
        kind: 'effect',
        effect: { kind: 'load_user', id: cmd.userId },
        continue: (result) => {
            if (result.kind !== 'user') throw new Error('wrong result type');
            if (result.user !== null) return { kind: 'done', value: err({ kind: 'already_exists' }) };
            return {
                kind: 'effect',
                effect: { kind: 'save_event', event: createOnboardedEvent(cmd) },
                continue: (_) => ({ kind: 'done', value: ok(cmd.userId) }),
            };
        },
    };
}

// Shell — the trampoline
async function executeSteps<T>(step: Step<T>, shell: Shell): Promise<T> {
    while (step.kind === 'effect') {
        const result = await shell.execute(step.effect);
        step = step.continue(result);
    }
    return step.value;
}
```

The Effect pattern is most valuable when the effect sequence is dynamic — when you need to conditionally branch between different I/O operations based on prior results, and you want to keep the branching logic pure and testable.
