---
id: TC-998
title: "A field read at a parent-typed slot reads the field by identity and the checked IR carries no conversion"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-359
    type: verifies
---
# TC-998: A field read at a parent-typed slot reads the field by identity and the checked IR carries no conversion

## Description

Verify that a child record value at a parent-typed slot is read by field identity, not position, and that check adds no node for it. Scope: FR-359-AC-1, FR-359-AC-2, FR-359-AC-3.

Catches an implementation that computes a field position from the parent and applies it to the child's key-sorted fields, which returns `promised_by` for `occurred_at`, and one that adds a supertype member or a conversion node to make the call check.

## Test Procedure

1. Declare `record OrderPlaced { order_id: UUID; occurred_at: Timestamp }`, `record ExpressOrderPlaced` specializing it with `promised_by: Timestamp`, and `f(e: OrderPlaced): Timestamp { e.occurred_at }` and `g(e: OrderPlaced): UUID { e.order_id }`. Check, emit and run both with an `ExpressOrderPlaced` whose three fields differ, and with an `OrderPlaced`.
2. Read the same fields through a record field `Holder.placed: OrderPlaced`, through an `OrderPlaced?` payload and through an element of `Sequence<OrderPlaced>`.
3. Add `h(e: OrderPlaced): Timestamp { e.promised_by }`.
4. Redeclare the child so it redefines `occurred_at` with a narrower type, and read `occurred_at` and `order_id` at the parent-typed parameter.
5. Emit the step 1 package, read it back, and emit the same program without `ExpressOrderPlaced`. Compare the `quire.op.record.project` application, the `field` member, the call node and every node id.
6. Hand-build a checked package whose `Field` read names `occurred_at` on a value whose declaration has no such field, and run it.

## Expected Results

- Step 1 returns the value's own `occurred_at` and `order_id` for the child and for the parent, never `promised_by`.
- Step 2 returns the same fields at every depth, and the value read back keeps `ExpressOrderPlaced` as its most-specific type.
- Step 3 refuses `ill_typed`/`operator-ineligible` at the `promised_by` member of `h`'s body.
- Step 4 returns the child's redefining slot for `occurred_at` and the inherited slot for `order_id`, with the type check gave them.
- Step 5 holds one `quire.op.record.project` with `field` `{declaration: OrderPlaced's node, name: occurred_at}`, no `convert` node, no `supertypes` member on any node, `Verified` on read, and equal read and call node ids in both packages.
- Step 6 returns the checked invariant `field-slot-unresolved`.
- Assertions compare typed values, causes and member paths, never message text.