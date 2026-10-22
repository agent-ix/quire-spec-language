---
id: TC-1164
title: "An instance's automaton-state key context binds its population and key"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: verifies
---
# TC-1164: An instance's automaton-state key context binds its population and key

## Description

Verify that the FR-338 key context of an `over` instance binds the
`["instance", population, key]` pair, so two populations sharing a key give
two distinct contexts, and that a reference value in its place is refused.

Scope: FR-338-AC-9.

## Test Procedure

1. Build a subject with populations `P1` and `P2` of `Counter`, each with
   universe `["a"]`, and the claim `always holds(c.value <= 1000)` with
   `over (c: Counter)`.
2. Encode each instance's `context` and compare the octets with the
   expected arrays built from each population's declaration identity text.
3. Check a closure certificate whose product state carries `P1`'s context
   against `P2`'s instance.
4. Replace the instance array of a product state's key with the key
   object's FR-181 reference value, then with the instance array plus an
   object type member, and check the certificate.

Tag the tests `#[trace("TC-1164", "FR-338-AC-9")]`.

## Expected Results

- Step 2: two contexts whose octets differ exactly in the population text.
- Step 3: the state is a foreign context and is not admitted.
- Step 4: each certificate is rejected `ShapeMismatch` at `ProductState`.
