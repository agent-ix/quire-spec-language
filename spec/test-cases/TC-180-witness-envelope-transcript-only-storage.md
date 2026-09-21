---
id: TC-180
title: "The witness envelope stores the transcript once and derives every other fact from it"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: verifies
---
# TC-180: The witness envelope stores the transcript once and derives every other fact from it

## Description

Verify that the counterexample/witness envelope's witness-shape state is
exactly one field (the admitted transcript), so that `harness_symbol()`,
`concrete_values()` and `decode()` have no second, independently-stored
value they could disagree with. The exhaustive field-count check (step 1)
is the falsifier: repeated same-input calls returning identical results
(steps 2-4) pass just as well under a construction-time cache that computes
and stores `concrete_values()` once alongside the transcript, since a cache
populated once and never invalidated still returns the same answer on every
call. Only counting the type's own fields — including private ones —
catches that case, because a cached-derived-fact field is a second field
regardless of its visibility. A wrong implementation this test would catch:
an envelope with a private `cached_values: OnceCell<Vec<WitnessValue>>`
field (or equivalent) set once at construction, which every repeated-call
comparison in steps 2-4 would pass, and which step 3's public-surface-only
inspection (present in an earlier draft of this test case) would also miss.
Scope: FR-070-AC-1.

## Test Procedure

1. From within the envelope type's own defining module (where private
   fields are visible — the idiomatic Rust place for a type's own unit
   tests), write an exhaustive struct-destructuring pattern naming only the
   transcript field and no other, with no `..` rest pattern, for example
   `let Witness { transcript } = value;`. Confirm this compiles against the
   envelope type as shipped.
2. Call `harness_symbol()`, `concrete_values()` and `decode()` twice each,
   with no mutation of the envelope between calls, and confirm each pair of
   same-method calls returns identical results.
3. Construct a second envelope directly from the same transcript bytes via
   a distinct code path (e.g. deserialization instead of the in-process
   constructor) and compare its derived facts against the first envelope's.
4. Construct a third envelope from a transcript that differs from the first
   two only in the concrete value one assertion binds, and confirm its
   `concrete_values()` differs from the first two envelopes' accordingly —
   showing the derived value tracks the transcript that was actually
   stored, not a value fixed at some other point in construction.

## Expected Results

- Step 1 compiles: the envelope type has exactly one field, so no exhaustive
  no-`..` destructuring naming only `transcript` can fail to compile, and no
  second field — cached or otherwise — exists for a future disagreement to
  hide in.
- Each pair of same-method calls in step 2 returns identical results.
- The two envelopes built by different paths in step 3, from identical
  transcript bytes, produce identical derived facts.
- The third envelope's derived value in step 4 differs from the first two's,
  tracking its own distinct transcript.
