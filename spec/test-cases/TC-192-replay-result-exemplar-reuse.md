---
id: TC-192
title: "#217's function exemplar builds on the existing result/request/witness types with no new type"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-072
    type: verifies
---
# TC-192: #217's function exemplar builds on the existing result/request/witness types with no new type

## Description

Verify that a function-application replay exemplar — standing in for
#217's real integration — can be built and compared using only FR-069's
proof-result envelope, FR-070's witness envelope, FR-071's request type and
FR-072's result type, with no new witness or replay type defined in the
exemplar's own scope. A wrong implementation this test would catch: a
result type whose `Witness`-arm payload is hard-coded to a shape only the
state family needs (for example, requiring a `frame_id` field every
function-application result would have to fake), forcing #217 to define a
parallel, function-specific result type instead of reusing this one, which
is exactly the "two authoritative producer paths" outcome ADR-011 §4
forbids. Scope: FR-072-AC-4.

## Test Procedure

1. Using only the four types built by FR-069 through FR-072, construct a
   minimal function-application scenario: a proof-result envelope for a
   `Counterexample` Kani run, a witness envelope decoding a two-argument
   function call's counterexample, a replay request naming the function by
   `QualifiedName`, and a replay result comparing the proved and replayed
   verdicts.
2. Diff the exemplar's own crate/module against its state before step 1
   and list every new `struct`, `enum`, or `trait` item the diff adds;
   confirm none of them has a field or variant typed to carry a proof
   category, a transcript, a witness value, a replay verdict, or a
   settlement — the identity-carrying content FR-069 through FR-072
   already type — as distinct from ordinary test glue (fixture builders,
   `#[test]` functions, assertion helpers) that hold no such field.
3. Repeat step 1 for a second, structurally different function (different
   arity, different argument types) and confirm the same four types are
   reused unchanged.

## Expected Results

- The scenario in step 1 is fully representable using only the FR-069
  through FR-072 types.
- Step 2's diff lists zero new items carrying proof-category, transcript,
  witness-value, replay-verdict, or settlement content; any new items
  found are ordinary test glue and carry no such field.
- Step 3's structurally different function reuses the identical four types
  with no modification.
