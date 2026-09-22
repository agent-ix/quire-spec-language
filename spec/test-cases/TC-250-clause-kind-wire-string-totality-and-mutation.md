---
id: TC-250
title: "Clause-kind wire-string totality in both directions, with mutation coverage"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: verifies
---
# TC-250: Clause-kind wire-string totality in both directions, with mutation coverage

## Description

ADR-013 O-10's validation row requires "wire-string totality tests in both
directions; mutation tests on each mapping." This test enumerates that
requirement concretely for the QSL checked clause-kind enum's own v2
mapping (the QSL → v2 direction; downstream layers own their own mappings
and their own tests, ADR-013 O-10). Scope: FR-088-AC-4.

## Test Procedure

1. Enumerate every variant of the checked clause-kind enum.
2. For each variant, call the forward mapping function and record its v2
   `node_tag`/`semantic_form`/clause-operation-identity output.
2a. Define a fixed expected table (variant → its exact wire-string
   spelling), written independently of the mapping function under test, and
   assert step 2's recorded output equals this table exactly, variant by
   variant. This is the direct check that catches a mutant swapping two
   variants' target strings (step 6(b)): totality and injectivity alone
   stay green under such a swap, since the codomain is merely permuted, not
   narrowed.
3. Confirm every variant maps to exactly one wire spelling, and no two
   variants map to the same wire spelling (forward totality and
   injectivity).
4. Enumerate every v2 wire spelling this enum's vocabulary defines
   (`quire.op.temporal.clause`, `quire.op.claim.clause`,
   `quire.op.protocol.control`, `quire.op.state.transition`, and the
   `node_tag`/`semantic_form` strings each names).
5. For each wire spelling, confirm it maps back (by the read/parse
   direction, or by inverting step 2's recorded map) to exactly one
   variant, with no wire spelling left unclaimed and none claimed twice
   (backward totality).
6. Mutation test: for the forward mapping function, apply each of the
   following mutants in turn and confirm the totality test from step 3
   fails for that mutant: (a) delete one match arm (forcing a fallback or
   compile error — record which); (b) swap two variants' target wire
   strings; (c) map one variant to a wire string another variant also
   claims (introducing a forward collision).

## Expected Results

- Step 2a: every variant's recorded output equals the fixed expected
  table's entry exactly; any mismatch fails this step and names the
  variant.
- Steps 2-3: forward mapping is total and injective; a gap or collision
  fails this step.
- Steps 4-5: backward mapping is total and injective over the same
  vocabulary; an unclaimed or double-claimed wire spelling fails this step.
- Step 6: mutants (a) and (c) are caught by the totality/injectivity test
  (step 3); mutant (b) (swapped target strings) is caught by step 2a's
  fixed-table equality check, not by step 3 alone, since a swap changes no
  variant's totality or injectivity, only which wire string a variant maps
  to. A mutant that survives both step 2a and step 3 fails this test and
  names the surviving mutant.
