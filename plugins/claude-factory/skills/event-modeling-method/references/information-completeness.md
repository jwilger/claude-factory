# Information Completeness Check

The information completeness check is a core verification step during event modeling. It answers: "does this slice have all the information it needs to do its job?"

## The check

For every element in the model, trace WHERE each required piece of information comes from:

- **Commands**: every field in the command must come from a visible UI element (screen), a read model already on screen, or a system-generated value (UUID, timestamp). If a command field has no visible source, draw a red arrow — information is missing.
- **Events**: every field in the event must come from the command that triggered it, the existing state (read from previous events), or a business rule computation. If an event field has no traceable source, the model is incomplete.
- **Read models**: every field displayed on screen must be derivable from the events in the store. If a screen shows data that no event provides, the slice is missing an event or an event field.

## The backwards question

Work right to left through the model: "For this event to have happened, what command must have been issued?" Then: "For that command to have been issued, what must the user have seen on screen?"

This backwards trace surfaces hidden state dependencies — data that the UI must display but that no current slice produces.

## Example

```
Screen: "Order Summary" displays order total
  ↓ (user sees total, clicks "Confirm")
Command: ConfirmOrder { orderId: UUID, total: Money }
  ↓ (system validates and emits)
Event: OrderConfirmed { orderId: UUID, total: Money, confirmedAt: Timestamp }
```

Completeness check:
- `orderId` in command: ✓ comes from URL param / screen state
- `total` in command: ✓ comes from Order Summary read model on screen
- `confirmedAt` in event: ✓ system-generated timestamp
- Order Summary read model: needs `total` — which event produces it? → `OrderItemAdded` events aggregated

If `OrderItemAdded` doesn't exist yet, draw it on the model and ask: what command creates it?

## Red arrows

In a modeling session, draw a red arrow from a command or event field to indicate "this information has no visible source." Red arrows are not problems — they're discoveries. Each red arrow becomes a question for the business:

- Is this value user-provided? Then we need a screen element.
- Is this value derived from existing events? Then we need to verify the projection.
- Is this a system-generated value? Then document that explicitly.

A completed model has no red arrows.

## Swimlane structure

The event model's horizontal swimlanes separate concerns:

| Lane | Content | Color |
|---|---|---|
| UI / Screens | Wireframes showing user-visible information | (any) |
| Read Models | Green stickies — data projected from events to feed screens | Green |
| Commands | Blue stickies — user intentions, triggering state changes | Blue |
| Events | Orange stickies — facts that happened, the durable record | Orange |
| Automations | Purple stickies — system-triggered commands, background processes | Purple |

Time flows left to right. The information completeness check traces vertical arrows between lanes — every arrow must have a source.

## Scenarios below slices

Place Given/When/Then scenarios directly below the slice they belong to. This keeps business rules colocated with the model elements they constrain:

```
GIVEN [precondition events that establish system state]
WHEN  [the command or trigger]
THEN  [the resulting events OR an error]
```

Scenarios for the happy path and all important error cases. Scenarios are human-readable — business stakeholders can write and review them without technical knowledge. They become the specification for the TDD behavioral tests in the Development phase.
