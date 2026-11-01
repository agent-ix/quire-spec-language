---
id: TC-1195
title: "State keys and transition identities encode every FR-181 value family"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: verifies
  - target: ix://agent-ix/quire-specification/FR-181
    type: references
---
# TC-1195: State keys and transition identities encode every FR-181 value family

## Description

Verify that `ModelSystem` writes decimal, quantity, UUID, Timestamp, enum,
record, tuple and union values in QSpec FR-181's typed canonical form, in
state keys, transition identities and step effects; that set elements take
FR-181's canonical element encoding; that a population's objects key as a
set of references independent of insertion order; and that a value of a
population type refuses `EncodingRefusal`.

Scope: FR-120-AC-16 to FR-120-AC-18.

## Test Procedure

1. Build FR-120-AC-16's `Item` model with its one initial state, compute the
   state key and read each field's bytes. Explore `setPrice` from that state
   and read the `1.50` transition's argument and step effect.
2. Add FR-120-AC-17's `prices`, `times` and `members` fields, build the
   state holding `prices = {1.50}` and `times = {2, 10}`, and build the two
   states whose `members` set is inserted as `i1, i2` and as `i2, i1`.
3. Build a state, a transition and a step effect that each hold a value of a
   population type; explore and replay through each.

Tag the tests `#[trace("TC-1195", "FR-120-AC-16")]`,
`#[trace("TC-1195", "FR-120-AC-17")]` and
`#[trace("TC-1195", "FR-120-AC-18")]`.

## Expected Results

- Step 1: the FR-181-AC-19 bytes for each family, with each enum, record,
  tuple and union `name` equal to the RFC 8785 text of its
  `{package, node}` declaration key, and the quantity `unit` equal to its
  `UnitId` text; the quantity remains in the `rational` form, and the decimal
  argument and effect retain their supplied coefficient and scale; no refusal.
- Step 2: the normalized `prices` element, the retained `price`, the
  byte-ordered `times` set, and equal state keys for the two `members`
  orders.
- Step 3: `NotSimulated::KeyEncoding` and `ReplayError::KeyEncoding`, each
  `invalid_runtime_input`/`invalid-value` naming the population-typed
  member; no panic and no state recorded for the value.
