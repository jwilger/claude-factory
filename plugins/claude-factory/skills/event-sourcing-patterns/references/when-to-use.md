# Event Sourcing Patterns — When to Use Each

A decision guide for choosing the right projection/read-model pattern.

## Decision tree

```
Need to query data?
├── Query is for a single aggregate's own state
│   └── Live Model (pattern 2) — replay from the aggregate's event stream; fast for short streams
│       └── Stream is growing long (thousands of events)?
│           └── Snapshots (pattern 5) — cache the projected state periodically
│
├── Query is across multiple aggregates or needs joins
│   └── Database Projected Read Model (pattern 1) — async projection to a SQL table
│       └── Need strong consistency (read your own writes)?
│           └── Partially Synchronous Projection (pattern 3) — project in the same transaction
│
└── Query needs computed/derived values (totals, status flags, aggregations)?
    └── Logic Read Model (pattern 4) — embed business logic in the projection

Need to coordinate a multi-step process?
├── Steps are independent and can be retried individually
│   └── Processor-TODO-List (pattern 6) — each step creates/closes a task row
│
└── Need to prevent duplicate resource allocation across concurrent commands?
    └── Reservation Pattern (pattern 7) — reserve before committing

Need to join event data with slow-changing reference data?
└── Lookup Tables (pattern 8) — small side tables populated by projections; keep local to each slice
```

## Common mistakes

**Using Live Model for cross-aggregate queries**: Live Model replays ONE aggregate's stream. Querying across aggregates requires loading multiple streams — use Database Projected Read Model instead.

**Making Lookup Tables global**: Sharing a lookup table between slices introduces coupling. Copy the table per slice; the duplication is worth the isolation.

**Snapshots as a first resort**: Snapshots add complexity (cache invalidation, replay-from-snapshot logic, schema evolution). Prefer closing the stream naturally (business concept like "settling accounts") before reaching for snapshots.

**Processor-TODO-List without idempotency**: Tasks must be idempotent — a task re-run (after a crash or retry) must not double-apply the effect. Use a unique task ID and check for completion before executing.

**Partially Synchronous Projection without measuring**: This pattern limits command throughput to projection speed. Always measure before choosing it — eventual consistency is almost always acceptable and much cheaper.

## Pattern compatibility

| Slice kind | Compatible patterns |
|---|---|
| `state_change` | All — state changes produce events that any pattern can consume |
| `state_view` (single aggregate) | Live Model, Snapshots, Logic Read Model |
| `state_view` (cross-aggregate) | Database Projected Read Model, Logic Read Model |
| `automation` | Processor-TODO-List, Reservation (if reserving resources) |
| `translation` | Database Projected Read Model for intermediate state; Lookup Tables for mapping |

## Combining patterns

Patterns compose. A common combination:
- **Database Projected Read Model + Logic Read Model**: the DB projection stores raw event data; a Logic Read Model wraps it with computed fields (e.g., computed total from stored line items)
- **Processor-TODO-List + Reservation**: the TODO processor reserves the resource before marking the task complete; if reservation fails, the task remains open for retry
- **Live Model + Snapshots**: start with Live Model; add Snapshots when profiling shows the replay is too slow

## Cost of eventual consistency

The Database Projected Read Model is eventually consistent — the projection lags behind the command. The lag is typically milliseconds to seconds in practice. This is acceptable for almost all read paths. Only choose Partially Synchronous Projection when:
1. The user must immediately see their own write (e.g., a shopping cart that shows the item they just added)
2. AND you have measured that eventual consistency breaks a specific user journey
3. AND the throughput impact has been profiled and accepted
