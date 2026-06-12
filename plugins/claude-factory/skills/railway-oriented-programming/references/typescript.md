# Railway-Oriented Programming — TypeScript

## neverthrow — idiomatic ROP for TypeScript

```bash
npm install neverthrow
```

```typescript
import { ok, err, Result, ResultAsync } from 'neverthrow';

// Domain error types — discriminated unions
type RegistrationError =
    | { readonly kind: 'email_taken' }
    | { readonly kind: 'invalid_email'; readonly value: string }
    | { readonly kind: 'weak_password'; readonly reason: string };

// Fallible domain function — Result in the return type, never throws
function registerUser(
    email: EmailAddress,
    password: string,
): Result<UserId, RegistrationError> {
    if (!isValidPassword(password)) {
        return err({ kind: 'weak_password', reason: 'must be at least 12 chars' });
    }
    const id = generateUserId();
    return ok(id);
}
```

## Chaining with andThen and map

```typescript
function processRegistration(
    rawEmail: string,
    rawPassword: string,
): Result<RegistrationConfirmation, RegistrationError | EmailParseError> {
    return parseEmail(rawEmail)           // Result<EmailAddress, EmailParseError>
        .andThen(email => checkAvailability(email))  // Result<EmailAddress, RegistrationError>
        .andThen(email => registerUser(email, rawPassword))  // Result<UserId, RegistrationError>
        .map(userId => buildConfirmation(userId));    // Result<RegistrationConfirmation, RegistrationError>
}
```

## ResultAsync for async operations

```typescript
import { ResultAsync } from 'neverthrow';

function saveOrder(order: Order): ResultAsync<OrderId, DatabaseError> {
    return ResultAsync.fromPromise(
        db.orders.insert(order),
        (e) => ({ kind: 'database_error' as const, cause: e }),
    );
}

// Chain async and sync results seamlessly
async function placeOrder(cmd: PlaceOrderCommand): Promise<Result<OrderId, PlaceOrderError>> {
    return parseOrder(cmd)                  // Result<Order, ValidationError>
        .asyncAndThen(order => saveOrder(order))  // ResultAsync<OrderId, DatabaseError>
        .match(
            (id) => ok(id),
            (e) => err(toPlaceOrderError(e)),
        );
}
```

## Pattern: exhaustive match at the boundary

```typescript
// At the application boundary, match every error to a response — no unchecked cases
const result = await placeOrder(command);
result.match(
    (orderId) => response.json({ orderId }),
    (error) => {
        switch (error.kind) {
            case 'validation_error': return response.status(400).json({ message: error.message });
            case 'product_not_found': return response.status(404).json({ message: 'Product not found' });
            case 'database_error': return response.status(503).json({ message: 'Service unavailable' });
            // No default — exhaustiveness enforced by @typescript-eslint/switch-exhaustiveness-check
        }
    }
);
```

## What NOT to do

```typescript
// Wrong — exceptions escape the type system
async function processPayment(amount: Money): Promise<PaymentId> {
    if (amount <= 0) throw new Error('invalid amount'); // invisible to callers
    // ...
}

// Wrong — undefined as error signal
function findUser(id: UserId): User | undefined { ... } // caller must remember to check

// Right — the return type communicates everything
function findUser(id: UserId): Result<User, UserNotFoundError> { ... }
```
