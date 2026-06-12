# Semantic Types — TypeScript

## Branded types — the pattern

TypeScript's structural type system means `type UserId = string` and `type ProductId = string` are the same type. Branded types (also called opaque types or nominal types) fix this:

```typescript
// The brand pattern
declare const __brand: unique symbol;
type Brand<B> = { readonly [__brand]: B };
type Branded<T, B> = T & Brand<B>;

// Domain types
type UserId = Branded<string, 'UserId'>;
type ProductId = Branded<string, 'ProductId'>;
type Quantity = Branded<number, 'Quantity'>;
type EmailAddress = Branded<string, 'EmailAddress'>;
```

These are zero-cost at runtime — they're only a compile-time constraint.

## Constructor functions (parse at the boundary)

```typescript
// Parse functions live at the I/O boundary — they're the only way to create the branded type
function parseUserId(raw: string): UserId {
    if (!raw || raw.trim().length === 0) throw new Error('UserId must not be empty');
    return raw as UserId;
}

// Or return Result instead of throwing:
function parseEmailAddress(raw: string): Result<EmailAddress, ValidationError> {
    const trimmed = raw.trim().toLowerCase();
    if (!EMAIL_RE.test(trimmed)) return err(new ValidationError(`invalid email: ${raw}`));
    return ok(trimmed as EmailAddress);
}

// After parsing, the type is proof — no re-validation needed
function sendWelcomeEmail(to: EmailAddress): Promise<void> { ... }
```

## Using zod for schema + branded types together

```typescript
import { z } from 'zod';

const UserIdSchema = z.string().min(1).uuid().brand<'UserId'>();
const EmailSchema = z.string().email().toLowerCase().trim().brand<'EmailAddress'>();

type UserId = z.infer<typeof UserIdSchema>;        // Branded<string, 'UserId'>
type EmailAddress = z.infer<typeof EmailSchema>;  // Branded<string, 'EmailAddress'>

// The zod parse IS the boundary — use it at deserialization, never sprinkle .safeParse() everywhere
const userId = UserIdSchema.parse(json.userId);   // throws on invalid
```

## Common pitfalls

```typescript
// Wrong — string slips through
function createOrder(userId: string, productId: string) { ... }
createOrder(product.id, user.id);  // transposed — compiles, runtime bug

// Right — compiler catches transposition
function createOrder(userId: UserId, productId: ProductId) { ... }
createOrder(product.id, user.id);  // TS error: ProductId not assignable to UserId
```

## Object shapes with branded fields

```typescript
// Every field carries its domain meaning
type Order = {
    id: OrderId;
    customerId: CustomerId;       // not string — cannot accidentally use userId here
    productId: ProductId;
    quantity: Quantity;           // not number — cannot accidentally use price here
    totalPrice: Money;            // not number — different semantic entirely
};
```

## Discriminated unions for domain state

```typescript
// State machine variants with branded payloads
type PaymentState =
    | { readonly status: 'pending'; readonly orderId: OrderId }
    | { readonly status: 'authorized'; readonly orderId: OrderId; readonly authCode: AuthCode }
    | { readonly status: 'captured'; readonly orderId: OrderId; readonly captureId: CaptureId }
    | { readonly status: 'failed'; readonly orderId: OrderId; readonly reason: FailureReason };

// Exhaustive switch is enforced by the @typescript-eslint/switch-exhaustiveness-check rule
```
