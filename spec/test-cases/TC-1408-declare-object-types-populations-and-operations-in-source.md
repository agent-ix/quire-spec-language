---
id: TC-1408
title: "Declare object types, populations and operations in QSL source"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-569
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: verifies
---
# TC-1408: Declare object types, populations and operations in QSL source

## Description

Verify that a unit's own source declares object types with fields, identity,
supertypes, populations and operations; that they resolve, refuse
collisions and check as a domain package's object types do; that S4 emits
them as source-owned `model`/`object_type` nodes with source-object identity
rows; and that every evaluation over them equals the evaluation over the same
types declared by a domain package. Scope: FR-569-AC-1 to FR-569-AC-10 and FR-105-AC-18.

Catches a source type that silently resolves to a model type of the same
name, a redefinition that skips conformance, a node id derived from member
content, and an evaluation whose value or charge log depends on who owns the
type.

## Test Procedure

Fixture **S** is this unit, after the header and `profile P =
"quire.state.core/v1";`:

```
object Order identity (total) {
  total: Int[0, 9];
  note: Text[0, 8; nfc]?;
  population orders;
  operation size(): Integer;
  operation count(): Integer;
  operation scaled(n: Integer): Integer;
  operation bump(): Boolean modifies self.total;
}
object Sub extends Order {
  operation size(): Integer redefines Order::size;
  operation scaled(m: Integer): Integer redefines Order::scaled;
}
abstract object Shape { population shapes; }

invariant I using P on Order at current { self.total >= 0 }
pre Q using P on Order::bump { self.total < 9 }
post R using P on Order::bump { self.total = pre(self.total) + 1 }
body B using P on Sub::size() { 7 }
body C using P on Order::size() { 1 }
```

Fixture **D**, its model twin, is a domain package `test/orders` selected as
`M` that declares the same object types, fields, identity, supertypes,
populations and operations (frames included), with S's state clauses and
bodies written `on M::Order`, `on M::Order::bump`, `on M::Sub::size()` and
`on M::Order::size()`.

1. Parse S; parse each malformed variant of AC-1. (AC-1)
2. Check S with the clauses of AC-2, alone and with a domain package that
   declares `Order` selected as `M`. (AC-2)
3. Check each collision variant of AC-3 and each declaration variant of AC-4,
   one change to S per run. (AC-3, AC-4)
4. Check S and D; compare their effective views, dispatch tables and the
   checked types of AC-5's expressions. (AC-5)
5. Emit S, S with V4's invariant added (S4) and D. Read the object,
   anchor, frame, clause-function and population-reference nodes, emit the
   AC-7 variants of S, and read S's package (which names no population) and
   D's with IR's strict reader. (AC-6, AC-7, FR-105-AC-18)
6. Read the `source_objects` rows of S's and D's emissions and of
   `identities.json`; run the refusal suppliers of AC-8. (AC-8)
7. Admit the snapshots of AC-9 over S, which selects no domain package.
   (AC-9)
8. Evaluate vectors V1 to V6 over S and D on the source route, with the
   current snapshot `orders` = {`o1` (`Order`, `total` 3), `o2` (`Sub`,
   `total` 4)} and, for V3 and V6, the invocation of `bump` on `o1` with the
   post snapshot `o1.total` 4: V1 `I` over `o1`; V2 `Q` over `o1`; V3 `R`;
   V4 `forall o in orders: deref(o).total >= 0` in an invariant; V5
   `r.size()` with `r` the reference to `o2`; V6 the same invocation with
   `o2.total` also changed in the post snapshot (a frame violation). (AC-10)

Tag the test `#[trace("FR-569-AC-1", ..., "FR-569-AC-10", "TC-1408")]`, each
test with the criteria it asserts.

## Expected Results

- Step 1: the forms and spans AC-1 states; one `invalid_syntax` per
  malformed variant, no `ObjectForm`.
- Step 2: the resolutions and refusals AC-2 states; `on Order` and
  `on M::Order` check against different field sets.
- Step 3: each variant refuses with the code, cause and spans AC-3 or AC-4
  states, and no checked package.
- Step 4: equal views, dispatch tables and types; the weakening `pre` is
  refused in both.
- Step 5: the node shapes of AC-6 and FR-105-AC-18; S's `Order` id is unchanged by the field
  addition, the operation rename and the move, and changed by the rename of
  `Order`; S's and D's `Order` ids differ.
- Step 6: three rows over S and none over D; the file round-trips equal;
  each refusal supplier gives the reason AC-8 states at the `Package` locus,
  with no view and no charge.
- Step 7: admission and refusals as AC-9 states.
- Step 8: V1 true, V2 true, V3 true, V4 true, V5 `7`, V6 `frame_violation`/
  `unauthorized-change` naming `o2.total` (FR-154), each with equal
  outcomes and equal ordered charge logs over S and D.
