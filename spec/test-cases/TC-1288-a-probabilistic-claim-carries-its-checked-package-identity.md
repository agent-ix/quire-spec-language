---
id: TC-1288
title: "A probabilistic claim carries its checked-package identity"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-498
    type: verifies
---
# TC-1288: A probabilistic claim carries its checked-package identity

## Description

Verify that emission writes one `probabilistic_claims` row per probabilistic claim
declaration, that `CheckedProbabilisticClaim` exposes the same identity, and that
the FR-421 causes carry it. Scope: FR-498-AC-1 to FR-498-AC-4.

## Test Procedure

1. Compile the QSpec TC-233 claims `PC-01` and `PC-02` (equal display names and
   spans) by reference, emit the package and read the rows; permute source order
   and re-emit.
2. Read each `CheckedProbabilisticClaim` and compare its identity with its row.
3. Check an elapsed-time claim with an unsupported form and a punctual interval
   in a non-admitted form over those claims and read the settled causes.
4. Check a claim without emission and compare its identity and cause; compare
   an emitted package with its rows against the graph of one without them.

Tag the test `#[trace("FR-498-AC-1", "FR-498-AC-2", "FR-498-AC-3", "FR-498-AC-4", "TC-1288")]`.
Vectors are referenced by QSpec case ID, never copied.

## Expected Results

- Step 1: two rows, two distinct declarations, equal across permutations.
- Step 2: each claim's identity equals its row.
- Step 3: each cause carries the selected claim's identity and member path.
- Step 4: the unemitted claim exposes no identity and keeps the span-locus refusal with no synthesized locus; the graphs are equal.
