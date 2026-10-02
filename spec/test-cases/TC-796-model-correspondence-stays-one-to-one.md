---
id: TC-796
title: "The model correspondence faults on a second, different entry in either direction"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-303
    type: verifies
---
# TC-796: The model correspondence faults on a second, different entry in either direction

## Description

Verify that `ModelCorrespondence` is one-to-one. Scope: FR-303-AC-1 to
FR-303-AC-4.

## Test Procedure

1. Record `(n, d1)`, then `(n, d2)`.
2. On a fresh correspondence, record `(n1, d)`, then `(n2, d)`.
3. On a fresh correspondence, record `(n, d)` twice.
4. Run spine `compile` on a unit through a test seam that makes S3 lowering
   record `(n, d1)` and then `(n, d2)`.

## Expected Results

1. The second record returns `runtime_invariant`/
   `established-invariant-broken` naming `n`, `d1`, `d2`; `n` resolves
   to `d1`.
2. The second record returns the same fault naming `n1`, `n2`, `d`; `n2`
   resolves to nothing.
3. Both records succeed; one entry for `n`.
4. The compile refuses `runtime_invariant`/`established-invariant-broken`,
   category internal failure, naming `n`, `d1`, `d2`; no checked package is
   returned.
