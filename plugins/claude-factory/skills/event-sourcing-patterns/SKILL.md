---
name: Event Sourcing Patterns
description: This skill should be used when implementing event-sourced systems, choosing how to project state from events, designing read models, or handling performance/consistency trade-offs in event-sourced applications. It covers the eight core implementation patterns from Martin Dilger's methodology.
version: 1.0.0
---

Event sourcing stores state as a sequence of events rather than a snapshot. Current state is derived by replaying the event stream. This enables full audit trails, temporal queries, and event-driven integration — but requires careful thought about how to project state efficiently.

## The eight patterns

**1. Database Projected Read Model**: Events are projected asynchronously into a separate database (e.g., PostgreSQL read table). Queries hit the read model, not the event store. Good for: complex queries, large data sets, reporting. Trade-off: eventual consistency (the projection lags behind the event stream).

**2. Live Model (Live Projection)**: State is projected in-memory at query time by replaying the relevant event stream. No separate read table. Good for: small event streams, strong consistency requirements, simple state. Trade-off: latency grows with stream length (mitigated by snapshots — see pattern 5).

**3. Partially Synchronous Projection**: Events are projected synchronously within the same transaction as the command, but the projection is still a separate table. Provides strong consistency without eventual consistency lag. Trade-off: command throughput is limited by projection speed.

**4. Logic Read Model**: Business logic is embedded in the projection. The read model computes derived values (totals, statuses, flags) rather than just mirroring events. Used when the view needs computed state that is expensive to recalculate on every read.

**5. Snapshots**: Periodically store a full snapshot of the current projected state alongside the event stream. On load: start from the most recent snapshot, replay only events since the snapshot. Reduces live-projection latency for long streams.

**6. Processor-TODO-List Pattern**: Events create work items in a processing table (a "TODO list"). A separate process works through the TODO list and marks items complete. Used for: saga/process manager patterns, retry logic, deferred work.

**7. Reservation Pattern**: Before emitting a success event, reserve the resource (e.g., unique slot, inventory unit). Reservations are transactional; the event is emitted only if the reservation succeeds. Used for: preventing double-booking, enforcing uniqueness across streams.

**8. Lookup Tables**: Side tables that map event data to domain concepts (e.g., product ID → price, user ID → role). Populated and updated by projections. Used when you need to join event-sourced data with external or slowly-changing reference data.

See `references/` for implementation examples per pattern in the factory's supported stacks.
