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

Verify that the counterexample/witness envelope stores the admitted
transcript as its only witness-shape field, and that `harness_symbol()`,
`concrete_values()` and `decode()` each recompute their result from that
one stored transcript on every call rather than from a separately-cached
copy set at construction time. A wrong implementation this test would
catch: an envelope that caches `concrete_values()`'s result in a field
alongside the transcript at construction time; if that cached copy and the
transcript were ever produced from two different sources (for example, a
future refactor that decodes once during construction and stores both), the
two could silently disagree, and this test's repeated-call comparison would
no longer hold. Scope: FR-070-AC-1.

## Test Procedure

1. Construct a witness envelope from a valid, admitted transcript naming a
   concrete counterexample.
2. Call `harness_symbol()`, `concrete_values()` and `decode()` twice each,
   with no mutation of the envelope between calls.
3. Inspect the envelope's public surface (via its type signature / a
   `compile_fail` probe) for any field other than the transcript that could
   hold a derived fact.
4. Construct a second envelope directly from the same transcript bytes via
   a distinct code path (e.g. deserialization instead of the in-process
   constructor) and compare its derived facts against the first envelope's.

## Expected Results

- Each pair of same-method calls in step 2 returns identical results.
- Step 3 finds no field other than the private transcript field that could
  hold a witness-shape fact.
- The two envelopes built by different paths in step 4, from identical
  transcript bytes, produce identical derived facts.
