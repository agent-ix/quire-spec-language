---
id: TC-426
title: "A stage limit names its kind, bound, actual counter and locus"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: verifies
---
# TC-426: A stage limit names its kind, bound, actual counter and locus

## Description

Verify FR-096's `LimitExceeded`: each kind has its `stage_limit_exceeded`
cause, and S2 and the family `check` each locate the limit where the charge
failed. This catches a limit with no locus where one is known, and a locus
invented for a synthesized function.

Scope: FR-096-AC-2, FR-096-AC-3, FR-096-AC-4, FR-096-AC-5.

## Test Procedure

1. Read `catalog_code()` of each `LimitKind` and of a `LimitExceeded` of
   each kind.
2. Run S2 with nesting-depth bound 8 over a body of `not`×8 `a`.
3. Check a declaration whose preimage input bytes exceed bound `B`; then
   reach the same limit for an FR-151 synthesized function.
4. Check a declaration whose work charge a work budget `W` denies.

Tag the tests `#[trace("TC-426", "FR-096-AC-n")]` with the AC each backs.

## Expected Results

- Step 1: `stage_limit_exceeded` with `input-bytes-exceeded`,
  `nesting-depth-exceeded`, `node-count-exceeded` and
  `work-budget-exceeded`, and each `LimitExceeded` gives its kind's code.
- Step 2: kind nesting depth, bound 8, actual 9, the region of the node at
  depth 9 under the unit's `RawSourceRef`.
- Step 3: kind input bytes, bound `B`, actual the measured bytes, the
  declaration's region; the synthesized function's limit has no locus.
- Step 4: kind work budget, bound `W`, actual the spend the denied charge
  would have reached, the declaration's region.

## Status

Planned. ADR-013 §7 slice S-5b (QSL-160), after S-4b and FR-091-AC-10.
