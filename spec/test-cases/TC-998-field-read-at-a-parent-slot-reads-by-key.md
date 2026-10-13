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

1. Declare `record OrderPlaced { order_id: UUID; occurred_at: Timestamp }`, `record ExpressOrderPlaced` specializing it with `promised_by: Timestamp`, `record RushOrderPlaced` specializing that with `rush_by: Timestamp`, and `f(e: OrderPlaced): Timestamp { e.occurred_at }` and `g(e: OrderPlaced): UUID { e.order_id }`. Check, emit and run both with an `ExpressOrderPlaced`, a `RushOrderPlaced` whose fields all differ, and an `OrderPlaced`.
2. Read the same fields through a record field `Holder.placed: OrderPlaced`, through an `OrderPlaced?` payload and through an element of `Sequence<OrderPlaced>`.
3. Add `h(e: OrderPlaced): Timestamp { e.promised_by }`.
4. Redeclare the child so it redefines `occurred_at` with a narrower record type, and read `occurred_at` and `order_id` at the parent-typed parameter. Redeclare a record with `Integer` field `n` and a child redefining `n` as `Int[0, 9]`, and read `n` at the parent-typed parameter.
5. Emit the step 1 package with an `OrderPlaced` domain event type beside it and read it back. Inspect the nodes.

## Expected Results

- Step 1 returns the value's own `occurred_at` and `order_id` for every one of the three types, never `promised_by` or `rush_by`.
- Step 2 returns the same fields at every depth, and the value read back keeps its own most-specific type.
- Step 3 refuses `ill_typed`/`type-mismatch` at the `promised_by` member of `h`'s body.
- Step 4 returns the child's redefining slot for `occurred_at`, the inherited slot for `order_id`, and the stored integer of `n` unchanged, admitted at `Integer`.
- Step 5 holds `OrderPlaced`, `ExpressOrderPlaced`, `RushOrderPlaced` and the event type each as a `model`/`record_value_type` node with a `ModelOwner` and none as a `composite_type`. `f`'s body holds one `quire.op.record.project` with `field` `{declaration: OrderPlaced's node, name: occurred_at}`. The call to `f` takes its argument by `ref` with no `convert` node between them. No node has a `supertypes` member. The package reads back `Verified`.
- Assertions compare typed values, causes, node tags and member paths, never message text or node ids.
