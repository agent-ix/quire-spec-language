---
id: TC-1164
title: "An instance's automaton-state key context binds its key"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: verifies
---
# TC-1164: An instance's automaton-state key context binds its key

## Description

Verify that the FR-338 key context of an `over` instance binds the
`["instance", key]` pair, so a key that two populations list gives one
context, distinct keys give distinct contexts, and a reference value or a
population member in its place is refused.

Scope: FR-338-AC-9.

## Test Procedure

1. Build a subject with populations `P1` and `P2` of `Counter`, each with
   universe `["a"]` and `a` admitted, and the claim
   `always holds(c.value <= 1000)` with `over (c: Counter)`; encode the
   instance's `context`; rename `P1`, then `P2`, and encode it again.
2. Build a subject with `P1` universe `["a"]` and `P2` universe `["b"]`;
   encode both instances' contexts and check a closure certificate whose
   product state carries `a`'s context against `b`'s instance.
3. Replace `["instance", "a"]` in a product state's key with, in turn, the
   key object's FR-181 reference value, `["instance", T, "a"]` (`T` the JSON
   string of `P1`'s declaration identity text) and
   `["instance", {"type":"text","value":"a"}]`, and check the certificate.
4. Over two subjects with population `P1` of `Counter` and universe
   `["a"]`, one whose initial state holds a `Counter` under `a` and one whose
   initial state holds a `Fast` (a subtype of `Counter`) under `a`, encode the
   instance's context in each.

Tag the tests `#[trace("TC-1164", "FR-338-AC-9")]`.

## Expected Results

- Step 1: one instance; its context is `[root, profile, "infinite", [[N, ["instance", "a"]]]]`, with equal octets before and after each rename.
- Step 2: two contexts whose octets differ exactly in the key; the state is a foreign context and is not admitted.
- Step 3: each certificate is rejected `ShapeMismatch` at `ProductState`.
- Step 4: the two contexts' octets are equal.
